use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::{
    domain::{
        CompatibilityState, Device, DriverCandidate, DriverSourceKind, InstallOptions,
        InstallPhase, InstallRecord, InstallResultState, InstallReview, InstallReviewItem,
        InstallSelection, InstallStatus, SignatureStatus,
    },
    inventory_store::InventoryStore,
    management::ActivityLog,
    operation_store::OperationStore,
    platform::{self, ElevatedInstallRequest, ElevatedRollbackRequest},
};

const REVIEW_LIFETIME_SECONDS: i64 = 10 * 60;

#[derive(Clone)]
struct PreparedItem {
    device: Device,
    candidate: DriverCandidate,
    download_url: String,
}

#[derive(Clone)]
struct PreparedInstall {
    expires_at: i64,
    items: Vec<PreparedItem>,
    options: InstallOptions,
}

#[derive(Clone)]
pub struct InstallManager {
    app_data_dir: PathBuf,
    reviews: Arc<Mutex<HashMap<String, PreparedInstall>>>,
    status: Arc<Mutex<InstallStatus>>,
    cancel: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    log: ActivityLog,
}

impl InstallManager {
    pub fn new(app_data_dir: PathBuf, log: ActivityLog) -> Self {
        Self {
            app_data_dir,
            reviews: Arc::new(Mutex::new(HashMap::new())),
            status: Arc::new(Mutex::new(InstallStatus::default())),
            cancel: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(false)),
            log,
        }
    }

    pub fn prepare(
        &self,
        inventory: &InventoryStore,
        selections: Vec<InstallSelection>,
        options: InstallOptions,
    ) -> Result<InstallReview, String> {
        if selections.is_empty() {
            return Err("Choose at least one recommended driver to install.".into());
        }
        if selections.len() > 25 {
            return Err("Install reviews are limited to 25 drivers at a time.".into());
        }
        let snapshot = inventory
            .latest_scan()?
            .ok_or_else(|| "Scan the machine before preparing an installation.".to_string())?;
        let mut items = Vec::with_capacity(selections.len());
        let mut review_items = Vec::with_capacity(selections.len());
        for selection in selections {
            let device = snapshot
                .devices
                .iter()
                .find(|device| device.instance_id == selection.device_instance_id)
                .cloned()
                .ok_or_else(|| {
                    "A selected device is no longer present in the latest scan.".to_string()
                })?;
            validate_candidate(&device, &selection.candidate)?;
            let download_url = selection
                .candidate
                .download_url
                .clone()
                .or(selection.resolved_download_url)
                .ok_or_else(|| {
                    format!(
                        "Resolve the package URL for {} before installing.",
                        selection.candidate.display_name
                    )
                })?;
            validate_download_url(selection.candidate.source, &download_url)?;
            review_items.push(InstallReviewItem {
                device_instance_id: device.instance_id.clone(),
                device_name: device.friendly_name.clone(),
                candidate_id: selection.candidate.id.clone(),
                candidate_name: selection.candidate.display_name.clone(),
                source: selection.candidate.source,
                version: selection.candidate.version.clone(),
                channel: selection.candidate.release_channel.clone(),
                current_version: device
                    .installed_driver
                    .as_ref()
                    .and_then(|driver| driver.version.clone()),
                download_size_bytes: selection.candidate.size_bytes,
                package_type: selection.candidate.package_type.clone(),
                restore_point_requested: options.create_restore_point,
                backup_requested: options.backup_current_package
                    && device.installed_driver.is_some(),
            });
            items.push(PreparedItem {
                device,
                candidate: selection.candidate,
                download_url,
            });
        }
        let now = now_seconds()?;
        let token = unique_id("review");
        let expires_at = now + REVIEW_LIFETIME_SECONDS;
        self.reviews
            .lock()
            .map_err(lock_error)?
            .retain(|_, review| review.expires_at > now);
        self.reviews.lock().map_err(lock_error)?.insert(
            token.clone(),
            PreparedInstall {
                expires_at,
                items,
                options,
            },
        );
        Ok(InstallReview {
            token,
            items: review_items,
            expires_at,
            warning: None,
        })
    }

    pub fn commit(
        &self,
        token: &str,
        app: AppHandle,
        inventory: InventoryStore,
        history: OperationStore,
    ) -> Result<String, String> {
        if self.active.swap(true, Ordering::SeqCst) {
            return Err("Another driver installation is already running.".into());
        }
        let plan = self
            .reviews
            .lock()
            .map_err(lock_error)?
            .remove(token)
            .ok_or_else(|| {
                self.active.store(false, Ordering::SeqCst);
                "The install review expired. Review the drivers again before installing."
                    .to_string()
            })?;
        if plan.expires_at <= now_seconds()? {
            self.active.store(false, Ordering::SeqCst);
            return Err(
                "The install review expired. Review the drivers again before installing.".into(),
            );
        }
        let operation_id = unique_id("install");
        self.cancel.store(false, Ordering::SeqCst);
        self.publish(
            &app,
            InstallStatus {
                operation_id: Some(operation_id.clone()),
                phase: InstallPhase::Downloading,
                progress: 0.0,
                current_item: plan
                    .items
                    .first()
                    .map(|item| item.device.friendly_name.clone()),
                completed_items: 0,
                total_items: plan.items.len(),
                message: "Starting managed download".into(),
                cancellable: true,
                reboot_required: false,
            },
        );
        let manager = self.clone();
        let returned_id = operation_id.clone();
        std::thread::spawn(move || {
            let result = manager.run_queue(&operation_id, plan, &app, &inventory, &history);
            if let Err(message) = result {
                let mut status = manager.current_status();
                status.phase = if manager.cancel.load(Ordering::SeqCst) {
                    InstallPhase::Cancelled
                } else {
                    InstallPhase::Failed
                };
                status.message = message;
                status.cancellable = false;
                manager.publish(&app, status);
            }
            manager.active.store(false, Ordering::SeqCst);
        });
        Ok(returned_id)
    }

    pub fn cancel(&self) -> Result<(), String> {
        let status = self.current_status();
        if !status.cancellable {
            return Err("The current installation phase cannot be cancelled safely.".into());
        }
        self.cancel.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn current_status(&self) -> InstallStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_default()
    }

    fn run_queue(
        &self,
        operation_id: &str,
        plan: PreparedInstall,
        app: &AppHandle,
        _inventory: &InventoryStore,
        history: &OperationStore,
    ) -> Result<(), String> {
        let total = plan.items.len();
        let mut reboot_required = false;
        for (index, item) in plan.items.into_iter().enumerate() {
            if self.cancel.load(Ordering::SeqCst) {
                return Err("Installation cancelled before Windows made system changes.".into());
            }
            let started_at = now_seconds()?;
            let record_id = unique_id("item");
            let item_root = self
                .app_data_dir
                .join("installations")
                .join(operation_id)
                .join(&record_id);
            fs::create_dir_all(&item_root).map_err(|error| {
                format!("Could not create the managed package directory: {error}")
            })?;
            let result = self.run_item(
                operation_id,
                (index, total),
                &item,
                &plan.options,
                &item_root,
                app,
            );
            let completed_at = now_seconds()?;
            let after = platform::enumerate_devices().ok().and_then(|devices| {
                devices
                    .into_iter()
                    .find(|device| device.instance_id == item.device.instance_id)
            });
            let (outcome, package_sha256, signature_verified) = match &result {
                Ok(value) => (
                    InstallResultState::Succeeded,
                    Some(value.sha256.clone()),
                    true,
                ),
                Err(error) if self.cancel.load(Ordering::SeqCst) => (
                    InstallResultState::Cancelled,
                    error.sha256.clone(),
                    error.signature_verified,
                ),
                Err(error) => (
                    InstallResultState::Failed,
                    error.sha256.clone(),
                    error.signature_verified,
                ),
            };
            let elevated = result.as_ref().ok().map(|value| &value.elevated);
            let message = result
                .as_ref()
                .map(|value| value.elevated.message.clone())
                .unwrap_or_else(|error| error.message.clone());
            let rollback_available = result.is_ok()
                && item
                    .device
                    .installed_driver
                    .as_ref()
                    .and_then(|driver| driver.published_inf_name.as_ref())
                    .is_some()
                && elevated
                    .and_then(|value| value.backup_path.as_ref())
                    .is_some()
                && result
                    .as_ref()
                    .is_ok_and(|value| value.package_kind == "inf");
            let record = InstallRecord {
                id: record_id,
                operation_id: operation_id.into(),
                started_at,
                completed_at,
                device_instance_id: item.device.instance_id.clone(),
                device_name: item.device.friendly_name.clone(),
                candidate_id: item.candidate.id.clone(),
                candidate_name: item.candidate.display_name.clone(),
                source: item.candidate.source,
                previous_version: item
                    .device
                    .installed_driver
                    .as_ref()
                    .and_then(|driver| driver.version.clone()),
                installed_version: after
                    .as_ref()
                    .and_then(|device| device.installed_driver.as_ref())
                    .and_then(|driver| driver.version.clone())
                    .or(item.candidate.version.clone()),
                previous_inf: item
                    .device
                    .installed_driver
                    .as_ref()
                    .and_then(|driver| driver.published_inf_name.clone()),
                package_sha256,
                signature_verified,
                restore_point_attempted: elevated
                    .is_some_and(|value| value.restore_point_attempted),
                restore_point_created: elevated.is_some_and(|value| value.restore_point_created),
                backup_path: elevated.and_then(|value| value.backup_path.clone()),
                state: outcome,
                message: message.clone(),
                reboot_required: elevated.is_some_and(|value| value.reboot_required),
                rollback_available,
            };
            history.save(&record)?;
            self.log.write(
                if record.state == InstallResultState::Succeeded {
                    "INFO"
                } else {
                    "ERROR"
                },
                "install",
                &format!("{}: {}", record.device_name, record.message),
            );
            match result {
                Ok(value) => {
                    reboot_required |= value.elevated.reboot_required;
                    self.publish(
                        app,
                        InstallStatus {
                            operation_id: Some(operation_id.into()),
                            phase: InstallPhase::Installing,
                            progress: (index + 1) as f64 / total as f64,
                            current_item: Some(item.device.friendly_name),
                            completed_items: index + 1,
                            total_items: total,
                            message,
                            cancellable: false,
                            reboot_required,
                        },
                    );
                }
                Err(error) => return Err(error.message),
            }
        }
        self.publish(
            app,
            InstallStatus {
                operation_id: Some(operation_id.into()),
                phase: InstallPhase::Completed,
                progress: 1.0,
                current_item: None,
                completed_items: total,
                total_items: total,
                message: if reboot_required {
                    format!("{total} driver package(s) installed. Restart required.")
                } else {
                    format!("{total} driver package(s) installed.")
                },
                cancellable: false,
                reboot_required,
            },
        );
        Ok(())
    }

    fn run_item(
        &self,
        operation_id: &str,
        position: (usize, usize),
        item: &PreparedItem,
        options: &InstallOptions,
        root: &Path,
        app: &AppHandle,
    ) -> Result<ItemSuccess, ItemFailure> {
        let (index, total) = position;
        let package_path = match self.download(operation_id, index, total, item, root, app) {
            Ok(path) => path,
            Err(message) => {
                return Err(ItemFailure {
                    message,
                    sha256: None,
                    signature_verified: false,
                });
            }
        };
        let sha256 = match sha256_file(&package_path) {
            Ok(hash) => hash,
            Err(message) => {
                return Err(ItemFailure {
                    message,
                    sha256: None,
                    signature_verified: false,
                });
            }
        };
        self.publish(
            app,
            InstallStatus {
                operation_id: Some(operation_id.into()),
                phase: InstallPhase::Verifying,
                progress: index as f64 / total as f64,
                current_item: Some(item.device.friendly_name.clone()),
                completed_items: index,
                total_items: total,
                message: "Verifying SHA-256 and Windows package signature".into(),
                cancellable: false,
                reboot_required: false,
            },
        );
        let (install_path, package_kind) = match prepare_verified_package(&package_path, root) {
            Ok(value) => value,
            Err(message) => {
                return Err(ItemFailure {
                    message,
                    sha256: Some(sha256),
                    signature_verified: false,
                });
            }
        };
        self.publish(
            app,
            InstallStatus {
                operation_id: Some(operation_id.into()),
                phase: InstallPhase::PreparingSafety,
                progress: index as f64 / total as f64,
                current_item: Some(item.device.friendly_name.clone()),
                completed_items: index,
                total_items: total,
                message:
                    "Requesting administrator approval for safety preparation and installation"
                        .into(),
                cancellable: false,
                reboot_required: false,
            },
        );
        let request = ElevatedInstallRequest {
            package_path: install_path,
            package_kind: package_kind.clone(),
            device_instance_id: item.device.instance_id.clone(),
            current_inf: item
                .device
                .installed_driver
                .as_ref()
                .and_then(|driver| driver.published_inf_name.clone()),
            backup_directory: root.join("backup"),
            create_restore_point: options.create_restore_point,
            backup_current_package: options.backup_current_package,
        };
        let elevated = platform::run_elevated_install(&request, &root.join("elevation")).map_err(
            |message| ItemFailure {
                message,
                sha256: Some(sha256.clone()),
                signature_verified: true,
            },
        )?;
        if !elevated.succeeded {
            return Err(ItemFailure {
                message: elevated.message,
                sha256: Some(sha256),
                signature_verified: true,
            });
        }
        Ok(ItemSuccess {
            sha256,
            package_kind,
            elevated,
        })
    }

    fn download(
        &self,
        operation_id: &str,
        index: usize,
        total: usize,
        item: &PreparedItem,
        root: &Path,
        app: &AppHandle,
    ) -> Result<PathBuf, String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30 * 60))
            .user_agent("DrvMatch/0.6.0")
            .build()
            .map_err(|error| format!("Could not initialize the package downloader: {error}"))?;
        let mut response = client
            .get(&item.download_url)
            .send()
            .map_err(|error| format!("Package download failed: {error}"))?;
        validate_download_url(item.candidate.source, response.url().as_str())?;
        if !response.status().is_success() {
            return Err(format!(
                "Package download returned HTTP {}.",
                response.status()
            ));
        }
        let expected = response.content_length().or(item.candidate.size_bytes);
        let filename = package_filename(&item.download_url);
        let partial = root.join(format!("{filename}.part"));
        let complete = root.join(filename);
        let mut file = File::create(&partial)
            .map_err(|error| format!("Could not create the managed package file: {error}"))?;
        let mut buffer = [0u8; 64 * 1024];
        let mut received = 0u64;
        loop {
            if self.cancel.load(Ordering::SeqCst) {
                let _ = fs::remove_file(&partial);
                return Err("Installation cancelled before Windows made system changes.".into());
            }
            let count = response
                .read(&mut buffer)
                .map_err(|error| format!("Package download was interrupted: {error}"))?;
            if count == 0 {
                break;
            }
            file.write_all(&buffer[..count])
                .map_err(|error| format!("Could not write the managed package: {error}"))?;
            received += count as u64;
            let item_progress = expected
                .filter(|size| *size > 0)
                .map(|size| (received as f64 / size as f64).min(1.0))
                .unwrap_or(0.0);
            self.publish(
                app,
                InstallStatus {
                    operation_id: Some(operation_id.into()),
                    phase: InstallPhase::Downloading,
                    progress: (index as f64 + item_progress) / total as f64,
                    current_item: Some(item.device.friendly_name.clone()),
                    completed_items: index,
                    total_items: total,
                    message: format!("Downloading {}", item.candidate.display_name),
                    cancellable: true,
                    reboot_required: false,
                },
            );
        }
        file.sync_all()
            .map_err(|error| format!("Could not finalize the managed package: {error}"))?;
        fs::rename(&partial, &complete)
            .map_err(|error| format!("Could not finalize the managed package: {error}"))?;
        Ok(complete)
    }

    fn publish(&self, app: &AppHandle, status: InstallStatus) {
        if let Ok(mut current) = self.status.lock() {
            *current = status.clone();
        }
        let _ = app.emit("install-status", status);
    }
}

struct ItemSuccess {
    sha256: String,
    package_kind: String,
    elevated: platform::ElevatedInstallResult,
}
struct ItemFailure {
    message: String,
    sha256: Option<String>,
    signature_verified: bool,
}

pub fn rollback(
    record_id: &str,
    app_data_dir: &Path,
    history: &OperationStore,
    log: &ActivityLog,
) -> Result<InstallRecord, String> {
    let mut record = history
        .load(record_id)?
        .ok_or_else(|| "The installation record was not found.".to_string())?;
    if !record.rollback_available
        || record.backup_path.is_none()
        || record.state != InstallResultState::Succeeded
    {
        return Err("Rollback is not available for this installation.".into());
    }
    let result = platform::run_elevated_rollback(
        &ElevatedRollbackRequest {
            device_instance_id: record.device_instance_id.clone(),
        },
        &app_data_dir.join("rollbacks").join(record_id),
    );
    record.completed_at = now_seconds()?;
    match result {
        Ok(result) if result.succeeded => {
            record.state = InstallResultState::RolledBack;
            record.message = result.message;
            record.reboot_required |= result.reboot_required;
            record.rollback_available = false;
        }
        Ok(result) => {
            record.state = InstallResultState::RollbackFailed;
            record.message = result.message;
        }
        Err(message) => {
            record.state = InstallResultState::RollbackFailed;
            record.message = message;
        }
    }
    history.save(&record)?;
    log.write(
        if record.state == InstallResultState::RolledBack {
            "INFO"
        } else {
            "ERROR"
        },
        "rollback",
        &format!("{}: {}", record.device_name, record.message),
    );
    Ok(record)
}

fn validate_candidate(device: &Device, candidate: &DriverCandidate) -> Result<(), String> {
    if candidate.compatibility.state != CompatibilityState::Compatible {
        return Err(
            "Only candidates with completed compatibility evidence can be installed.".into(),
        );
    }
    if matches!(candidate.signature, SignatureStatus::Unsigned) {
        return Err("Unsigned driver packages cannot be installed by DrvMatch.".into());
    }
    let matched = candidate
        .compatibility
        .matched_id
        .as_ref()
        .map(|value| value.to_ascii_uppercase());
    if let Some(matched) = matched {
        let known = device
            .hardware_ids
            .iter()
            .chain(device.compatible_ids.iter())
            .any(|id| id.eq_ignore_ascii_case(&matched));
        if !known {
            return Err(
                "The candidate's recorded hardware match does not belong to the selected device."
                    .into(),
            );
        }
    }
    Ok(())
}

fn validate_download_url(source: DriverSourceKind, value: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(value).map_err(|_| "The package URL is invalid.".to_string())?;
    if url.scheme() != "https" {
        return Err("Driver packages must be downloaded over HTTPS.".into());
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let allowed = match source {
        DriverSourceKind::WindowsUpdate | DriverSourceKind::MicrosoftCatalog => {
            host == "download.windowsupdate.com" || host.ends_with(".download.windowsupdate.com")
        }
        DriverSourceKind::Amd => host == "amd.com" || host.ends_with(".amd.com"),
        DriverSourceKind::Nvidia => host == "nvidia.com" || host.ends_with(".nvidia.com"),
        DriverSourceKind::Intel => host == "intel.com" || host.ends_with(".intel.com"),
    };
    if !allowed {
        return Err(format!(
            "The package host {host} does not match the selected official source."
        ));
    }
    Ok(())
}

fn prepare_verified_package(package: &Path, root: &Path) -> Result<(PathBuf, String), String> {
    let extension = package
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "cab" => {
            let extracted = root.join("extracted");
            fs::create_dir_all(&extracted).map_err(|error| {
                format!("Could not create the package staging directory: {error}")
            })?;
            let expand =
                PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into()))
                    .join("System32")
                    .join("expand.exe");
            let status = Command::new(expand)
                .arg("-F:*")
                .arg(package)
                .arg(&extracted)
                .status()
                .map_err(|error| format!("Could not extract the Catalog package: {error}"))?;
            if !status.success() {
                return Err("Windows could not extract the downloaded Catalog package.".into());
            }
            let infs = find_files(&extracted, "inf")?;
            if infs.is_empty() {
                return Err(
                    "The downloaded Catalog package contains no INF driver package.".into(),
                );
            }
            for inf in &infs {
                platform::verify_inf_signature(inf)?;
            }
            Ok((
                if infs.len() == 1 {
                    infs[0].clone()
                } else {
                    extracted
                },
                "inf".into(),
            ))
        }
        "inf" => {
            platform::verify_inf_signature(package)?;
            Ok((package.to_path_buf(), "inf".into()))
        }
        "exe" | "msi" => {
            platform::verify_file_signature(package)?;
            Ok((package.to_path_buf(), "vendor-installer".into()))
        }
        _ => Err(format!(
            "DrvMatch does not install .{extension} packages. Supported types are CAB, INF, EXE, and MSI."
        )),
    }
}

fn find_files(root: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("Could not inspect the staged package: {error}"))?
        {
            let path = entry
                .map_err(|error| format!("Could not inspect the staged package: {error}"))?
                .path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(extension))
            {
                result.push(path);
            }
        }
    }
    result.sort();
    Ok(result)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("Could not open the downloaded package for hashing: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not hash the downloaded package: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn package_filename(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| {
            url.path_segments()
                .and_then(|mut segments| segments.next_back().map(str::to_string))
        })
        .filter(|name| {
            !name.is_empty()
                && name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || ".-_()".contains(character)
                })
        })
        .unwrap_or_else(|| "driver-package.bin".into())
}

fn unique_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    format!("{prefix}-{nanos}-{}", std::process::id())
}

fn now_seconds() -> Result<i64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .map_err(|error| format!("System clock is before the Unix epoch: {error}"))
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> String {
    "The installation coordinator is unavailable.".into()
}

#[cfg(test)]
mod tests {
    use super::{package_filename, sha256_file, validate_download_url};
    use crate::domain::DriverSourceKind;

    #[test]
    fn hashes_managed_packages_with_sha256() {
        let path = std::env::temp_dir().join(format!("drvmatch-sha-{}", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(
            sha256_file(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn source_urls_are_https_and_allowlisted() {
        assert!(
            validate_download_url(DriverSourceKind::Amd, "https://drivers.amd.com/package.exe")
                .is_ok()
        );
        assert!(
            validate_download_url(DriverSourceKind::Amd, "http://drivers.amd.com/package.exe")
                .is_err()
        );
        assert!(
            validate_download_url(DriverSourceKind::Amd, "https://example.com/package.exe")
                .is_err()
        );
        assert_eq!(
            package_filename("https://example.com/path/driver.cab?x=1"),
            "driver.cab"
        );
    }
}
