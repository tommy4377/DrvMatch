use crate::domain::{
    CandidateCompatibility, CompatibilityState, Device, DriverCandidate, DriverSourceKind,
    SignatureStatus,
};

use super::DriverSource;

pub struct WindowsUpdateSource;

impl DriverSource for WindowsUpdateSource {
    fn kind(&self) -> DriverSourceKind {
        DriverSourceKind::WindowsUpdate
    }

    fn cache_namespace(&self) -> &'static str {
        "windows-update-applicable-v1"
    }

    fn cache_key(&self, _device: &Device) -> Option<String> {
        Some("current-machine".into())
    }

    fn ttl_seconds(&self) -> i64 {
        Self::CACHE_SECONDS
    }

    fn discover(
        &self,
        _device: &Device,
        retrieved_at: i64,
    ) -> Result<Vec<DriverCandidate>, String> {
        discover_applicable_drivers(retrieved_at)
    }
}

#[cfg(windows)]
fn discover_applicable_drivers(retrieved_at: i64) -> Result<Vec<DriverCandidate>, String> {
    use windows::{
        Win32::System::{
            Com::{
                CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
                CoUninitialize,
            },
            UpdateAgent::{
                IStringCollection, IUpdate, IUpdateDownloadContentCollection, IUpdateSession,
                IWindowsDriverUpdate, UpdateSession,
            },
        },
        core::{BSTR, Interface},
    };

    struct ComApartment;
    impl Drop for ComApartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    result
        .ok()
        .map_err(|error| format!("Could not initialize Windows Update COM: {error}"))?;
    let _apartment = ComApartment;

    let session: IUpdateSession = unsafe {
        CoCreateInstance(&UpdateSession, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("Could not create a Windows Update session: {error}"))?
    };
    unsafe {
        session
            .SetClientApplicationID(&BSTR::from(concat!("DrvMatch ", env!("CARGO_PKG_VERSION"))))
            .map_err(|error| format!("Could not identify the Windows Update client: {error}"))?;
    }
    let searcher = unsafe {
        session
            .CreateUpdateSearcher()
            .map_err(|error| format!("Could not create the Windows Update searcher: {error}"))?
    };
    let search_result = unsafe {
        searcher
            .Search(&BSTR::from(
                "IsInstalled=0 and Type='Driver' and IsHidden=0",
            ))
            .map_err(|error| format!("Windows Update driver search failed: {error}"))?
    };
    let updates = unsafe {
        search_result
            .Updates()
            .map_err(|error| format!("Could not read Windows Update search results: {error}"))?
    };
    let count = unsafe { updates.Count() }
        .map_err(|error| format!("Could not count Windows Update results: {error}"))?;
    let mut candidates = Vec::with_capacity(count.max(0) as usize);

    for index in 0..count {
        let update: IUpdate = unsafe { updates.get_Item(index) }
            .map_err(|error| format!("Could not read Windows Update item {index}: {error}"))?;
        let Ok(driver) = update.cast::<IWindowsDriverUpdate>() else {
            continue;
        };
        let identity = unsafe { update.Identity() }
            .map_err(|error| format!("Could not read Windows Update identity: {error}"))?;
        let update_id = unsafe { identity.UpdateID() }
            .map_err(|error| format!("Could not read Windows Update ID: {error}"))?
            .to_string();
        let revision = unsafe { identity.RevisionNumber() }.unwrap_or_default();
        let title = unsafe { update.Title() }
            .map(|value| value.to_string())
            .unwrap_or_else(|_| "Windows driver update".into());
        let hardware_id = unsafe { driver.DriverHardwareID() }
            .ok()
            .and_then(non_empty_bstr);
        let download_url = unsafe { update.DownloadContents() }
            .ok()
            .and_then(first_download_url);
        let release_notes_url = unsafe { update.MoreInfoUrls() }.ok().and_then(first_string);
        let support_url = unsafe { update.SupportUrl() }.ok().and_then(non_empty_bstr);
        let is_beta = unsafe { update.IsBeta() }
            .map(|value| value.as_bool())
            .unwrap_or(false);

        candidates.push(DriverCandidate {
            id: format!("windows-update:{update_id}:{revision}"),
            source: DriverSourceKind::WindowsUpdate,
            source_specific_id: format!("{update_id}:{revision}"),
            alternate_sources: vec![],
            display_name: title.clone(),
            provider: unsafe { driver.DriverProvider() }
                .ok()
                .and_then(non_empty_bstr),
            manufacturer: unsafe { driver.DriverManufacturer() }
                .ok()
                .and_then(non_empty_bstr),
            version: version_from_title(&title),
            version_is_package_version: false,
            driver_date: unsafe { driver.DriverVerDate() }
                .ok()
                .and_then(ole_date_to_unix),
            publication_date: None,
            class_name: unsafe { driver.DriverClass() }
                .ok()
                .and_then(non_empty_bstr),
            supported_os: vec![],
            supported_architectures: vec![],
            hardware_ids: hardware_id.into_iter().collect(),
            compatible_ids: vec![],
            download_url,
            details_url: support_url,
            release_notes_url,
            release_channel: is_beta.then(|| "Beta".into()),
            oem_models: vec![],
            known_issues: vec![],
            known_regressions: vec![],
            fixed_issues: vec![],
            security_relevant: false,
            signature: SignatureStatus::Unknown,
            package_type: Some("Windows Update driver".into()),
            package_group: None,
            size_bytes: None,
            retrieved_at,
            compatibility: CandidateCompatibility {
                state: CompatibilityState::NeedsReview,
                matched_id: None,
                match_kind: None,
                reasons: vec![],
            },
        });
    }

    fn non_empty_bstr(value: BSTR) -> Option<String> {
        let value = value.to_string();
        (!value.trim().is_empty()).then_some(value)
    }

    fn first_string(collection: IStringCollection) -> Option<String> {
        if unsafe { collection.Count() }.ok()? <= 0 {
            return None;
        }
        unsafe { collection.get_Item(0) }
            .ok()
            .and_then(non_empty_bstr)
    }

    fn first_download_url(collection: IUpdateDownloadContentCollection) -> Option<String> {
        if unsafe { collection.Count() }.ok()? <= 0 {
            return None;
        }
        unsafe { collection.get_Item(0) }
            .ok()
            .and_then(|content| unsafe { content.DownloadUrl() }.ok())
            .and_then(non_empty_bstr)
    }

    Ok(candidates)
}

#[cfg(not(windows))]
fn discover_applicable_drivers(_retrieved_at: i64) -> Result<Vec<DriverCandidate>, String> {
    Err("Windows Update discovery is only available on Windows.".into())
}

fn ole_date_to_unix(value: f64) -> Option<i64> {
    const UNIX_EPOCH_AS_OLE_DATE: f64 = 25_569.0;
    const SECONDS_PER_DAY: f64 = 86_400.0;
    value
        .is_finite()
        .then(|| ((value - UNIX_EPOCH_AS_OLE_DATE) * SECONDS_PER_DAY).round() as i64)
}

fn version_from_title(title: &str) -> Option<String> {
    title
        .rsplit(" - ")
        .next()
        .map(str::trim)
        .filter(|value| {
            !value.is_empty()
                && value.chars().all(|character| {
                    character.is_ascii_digit() || matches!(character, '.' | '-' | '_')
                })
                && value.chars().any(|character| character.is_ascii_digit())
        })
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{discover_applicable_drivers, ole_date_to_unix, version_from_title};

    #[test]
    fn converts_ole_automation_epoch() {
        assert_eq!(ole_date_to_unix(25_569.0), Some(0));
    }

    #[test]
    fn extracts_numeric_driver_version_only() {
        assert_eq!(
            version_from_title("Intel - Display - 31.0.101.5590").as_deref(),
            Some("31.0.101.5590")
        );
        assert_eq!(version_from_title("Driver update"), None);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires the live Windows Update service"]
    fn live_windows_update_query_completes() {
        discover_applicable_drivers(123).unwrap();
    }
}
