mod domain;
mod inventory_store;
mod metadata_cache;
mod platform;
mod ranking;
mod sources;

use domain::{CandidateDiscovery, DownloadResolution, InventorySnapshot, ScanSummary};
use inventory_store::InventoryStore;
use metadata_cache::MetadataCache;
use tauri::{
    Manager, State,
    window::{Effect, EffectsBuilder},
};

#[tauri::command]
async fn scan_inventory(store: State<'_, InventoryStore>) -> Result<InventorySnapshot, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let devices = platform::enumerate_devices()?;
        store.save_scan(devices)
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
    device_instance_id: String,
) -> Result<CandidateDiscovery, String> {
    let store = store.inner().clone();
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let snapshot = store
            .latest_scan()?
            .ok_or_else(|| "Scan the machine before checking driver sources.".to_string())?;
        let device = snapshot
            .devices
            .iter()
            .find(|device| device.instance_id == device_instance_id)
            .ok_or_else(|| "The selected device is not present in the latest scan.".to_string())?;
        Ok(sources::discover_microsoft_candidates(device, &cache))
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
fn set_acrylic(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "The main window is unavailable".to_string())?;

    let effects = enabled.then(|| EffectsBuilder::new().effect(Effect::Acrylic).build());
    window
        .set_effects(effects)
        .map_err(|error| format!("Could not update the window material: {error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let store = InventoryStore::open(&data_dir).map_err(std::io::Error::other)?;
            let cache = MetadataCache::open(&data_dir).map_err(std::io::Error::other)?;
            app.manage(store);
            app.manage(cache);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_inventory,
            list_scans,
            load_scan,
            discover_candidates,
            resolve_catalog_download,
            set_acrylic
        ])
        .run(tauri::generate_context!())
        .expect("error while running DrvMatch");
}
