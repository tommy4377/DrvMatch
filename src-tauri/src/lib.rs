mod domain;
mod installation;
mod inventory_store;
mod management;
mod metadata_cache;
mod operation_store;
mod platform;
mod ranking;
mod sources;

use std::time::Instant;

use domain::{
    AppInfo, AppSettings, CacheStats, CandidateDiscovery, DownloadResolution, InstallOptions,
    InstallRecord, InstallReview, InstallSelection, InstallStatus, InventorySnapshot, LogVerbosity,
    ScanSummary, SourceHealth,
};
use installation::InstallManager;
use inventory_store::InventoryStore;
use management::{ActivityLog, SettingsStore};
use metadata_cache::MetadataCache;
use operation_store::OperationStore;
use tauri::{
    Manager, State,
    window::{Effect, EffectsBuilder},
};

#[tauri::command]
async fn scan_inventory(
    store: State<'_, InventoryStore>,
    log: State<'_, ActivityLog>,
) -> Result<InventorySnapshot, String> {
    let store = store.inner().clone();
    let log = log.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let machine = platform::detect_machine_identity();
        let devices = platform::enumerate_devices()?;
        let snapshot = store.save_scan(machine, devices)?;
        log.write(
            "INFO",
            "inventory",
            &format!(
                "Scanned {} present devices in {} ms",
                snapshot.summary.device_count,
                started.elapsed().as_millis()
            ),
        );
        Ok(snapshot)
    })
    .await
    .map_err(|error| format!("Device inventory task failed: {error}"))?
}

#[tauri::command]
async fn list_scans(store: State<'_, InventoryStore>) -> Result<Vec<ScanSummary>, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || store.list_scans())
        .await
        .map_err(|error| format!("Scan history task failed: {error}"))?
}

#[tauri::command]
async fn load_scan(store: State<'_, InventoryStore>, id: i64) -> Result<InventorySnapshot, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || store.load_scan(id))
        .await
        .map_err(|error| format!("Stored scan task failed: {error}"))??
        .ok_or_else(|| format!("Scan {id} was not found"))
}

#[tauri::command]
async fn discover_candidates(
    store: State<'_, InventoryStore>,
    cache: State<'_, MetadataCache>,
    settings: State<'_, SettingsStore>,
    operations: State<'_, OperationStore>,
    log: State<'_, ActivityLog>,
    device_instance_id: String,
) -> Result<CandidateDiscovery, String> {
    let saved_settings = settings.get()?;
    let mut enabled_sources = saved_settings.enabled_sources;
    enabled_sources.extend(saved_settings.enabled_oem_sources);
    let detailed_logging = saved_settings.log_verbosity == LogVerbosity::Detailed;
    if enabled_sources.is_empty() {
        return Err("At least one driver source must remain enabled.".into());
    }
    let store = store.inner().clone();
    let cache = cache.inner().clone();
    let operations = operations.inner().clone();
    let log = log.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let snapshot = store
            .latest_scan()?
            .ok_or_else(|| "Scan the machine before checking driver sources.".to_string())?;
        let device = snapshot
            .devices
            .iter()
            .find(|device| device.instance_id == device_instance_id)
            .ok_or_else(|| "The selected device is not present in the latest scan.".to_string())?;
        let discovery =
            sources::discover_candidates(device, &snapshot.machine, &cache, &enabled_sources);
        operations.save_source_health(&discovery.sources)?;
        log.write(
            "INFO",
            "sources",
            &format!(
                "Checked {} sources for {} and found {} candidates in {} ms",
                discovery.sources.len(),
                device.friendly_name,
                discovery.candidates.len(),
                started.elapsed().as_millis()
            ),
        );
        if detailed_logging {
            for health in &discovery.sources {
                log.write(
                    "DETAIL",
                    "sources",
                    &format!(
                        "{:?}: {:?}; {} candidates; cached={}; {}",
                        health.source,
                        health.state,
                        health.candidate_count,
                        health.cached,
                        health.message.as_deref().unwrap_or("no additional message")
                    ),
                );
            }
        }
        Ok(discovery)
    })
    .await
    .map_err(|error| format!("Candidate discovery task failed: {error}"))?
}

#[tauri::command]
async fn resolve_catalog_download(
    cache: State<'_, MetadataCache>,
    update_id: String,
) -> Result<DownloadResolution, String> {
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        sources::resolve_catalog_download(&update_id, &cache)
    })
    .await
    .map_err(|error| format!("Download metadata task failed: {error}"))?
}

#[tauri::command]
async fn prepare_install(
    manager: State<'_, InstallManager>,
    store: State<'_, InventoryStore>,
    settings: State<'_, SettingsStore>,
    selections: Vec<InstallSelection>,
) -> Result<InstallReview, String> {
    let saved = settings.get()?;
    let options = InstallOptions {
        create_restore_point: saved.create_restore_point,
        backup_current_package: saved.backup_current_package,
    };
    manager.prepare(store.inner(), selections, options)
}

#[tauri::command]
async fn commit_install(
    app: tauri::AppHandle,
    manager: State<'_, InstallManager>,
    store: State<'_, InventoryStore>,
    history: State<'_, OperationStore>,
    token: String,
) -> Result<String, String> {
    manager.commit(&token, app, store.inner().clone(), history.inner().clone())
}

#[tauri::command]
fn cancel_install(manager: State<'_, InstallManager>) -> Result<(), String> {
    manager.cancel()
}

#[tauri::command]
fn get_install_status(manager: State<'_, InstallManager>) -> InstallStatus {
    manager.current_status()
}

#[tauri::command]
async fn list_install_history(
    history: State<'_, OperationStore>,
) -> Result<Vec<InstallRecord>, String> {
    let history = history.inner().clone();
    tauri::async_runtime::spawn_blocking(move || history.list())
        .await
        .map_err(|error| format!("Installation history task failed: {error}"))?
}

#[tauri::command]
async fn rollback_install(
    app: tauri::AppHandle,
    history: State<'_, OperationStore>,
    log: State<'_, ActivityLog>,
    record_id: String,
) -> Result<InstallRecord, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("The application data directory is unavailable: {error}"))?;
    let history = history.inner().clone();
    let log = log.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        installation::rollback(&record_id, &data_dir, &history, &log)
    })
    .await
    .map_err(|error| format!("Rollback task failed: {error}"))?
}

#[tauri::command]
fn get_settings(settings: State<'_, SettingsStore>) -> Result<AppSettings, String> {
    settings.get()
}

#[tauri::command]
fn save_settings(
    settings: State<'_, SettingsStore>,
    values: AppSettings,
    log: State<'_, ActivityLog>,
) -> Result<AppSettings, String> {
    let saved = settings.save(&values)?;
    log.write("INFO", "settings", "Application settings saved");
    Ok(saved)
}

#[tauri::command]
fn list_source_health(history: State<'_, OperationStore>) -> Result<Vec<SourceHealth>, String> {
    history.list_source_health()
}

#[tauri::command]
fn get_cache_stats(cache: State<'_, MetadataCache>) -> Result<CacheStats, String> {
    cache.stats()
}

#[tauri::command]
fn clear_metadata_cache(
    cache: State<'_, MetadataCache>,
    log: State<'_, ActivityLog>,
) -> Result<CacheStats, String> {
    let stats = cache.clear()?;
    log.write("INFO", "cache", "Source metadata cache cleared");
    Ok(stats)
}

#[tauri::command]
fn read_activity_log(log: State<'_, ActivityLog>) -> Result<String, String> {
    log.read_tail()
}

#[tauri::command]
fn clear_activity_log(log: State<'_, ActivityLog>) -> Result<(), String> {
    log.clear()
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        repository: "https://github.com/tommy4377/DrvMatch".into(),
    }
}

#[tauri::command]
fn set_acrylic(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "The main window is unavailable".to_string())?;

    let effects = enabled.then(|| EffectsBuilder::new().effect(Effect::Acrylic).build());
    window
        .set_effects(effects)
        .map_err(|error| format!("Could not update the window material: {error}"))
}

#[tauri::command]
fn get_windows_accent() -> Option<String> {
    platform::windows_accent_color()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let started = Instant::now();
            let data_dir = app.path().app_data_dir()?;
            let store = InventoryStore::open(&data_dir).map_err(std::io::Error::other)?;
            let cache = MetadataCache::open(&data_dir).map_err(std::io::Error::other)?;
            let operation_store = OperationStore::open(&data_dir).map_err(std::io::Error::other)?;
            let settings_store = SettingsStore::open(&data_dir).map_err(std::io::Error::other)?;
            let activity_log = ActivityLog::open(&data_dir).map_err(std::io::Error::other)?;
            activity_log.write(
                "INFO",
                "application",
                &format!("DrvMatch started in {} ms", started.elapsed().as_millis()),
            );
            let install_manager = InstallManager::new(data_dir, activity_log.clone());
            app.manage(store);
            app.manage(cache);
            app.manage(operation_store);
            app.manage(settings_store);
            app.manage(activity_log);
            app.manage(install_manager);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_inventory,
            list_scans,
            load_scan,
            discover_candidates,
            resolve_catalog_download,
            prepare_install,
            commit_install,
            cancel_install,
            get_install_status,
            list_install_history,
            rollback_install,
            get_settings,
            save_settings,
            list_source_health,
            get_cache_stats,
            clear_metadata_cache,
            read_activity_log,
            clear_activity_log,
            get_app_info,
            get_windows_accent,
            set_acrylic
        ])
        .run(tauri::generate_context!())
        .expect("error while running DrvMatch");
}

pub fn run_privileged_helper(args: &[String]) -> Option<i32> {
    #[cfg(windows)]
    {
        platform::run_privileged_helper(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        None
    }
}
