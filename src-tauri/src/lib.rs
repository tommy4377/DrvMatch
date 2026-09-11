mod domain;
mod platform;

use domain::Device;
use tauri::{
    Manager,
    window::{Effect, EffectsBuilder},
};

#[tauri::command]
async fn enumerate_devices() -> Result<Vec<Device>, String> {
    tauri::async_runtime::spawn_blocking(platform::enumerate_devices)
        .await
        .map_err(|error| format!("Device inventory task failed: {error}"))?
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
        .invoke_handler(tauri::generate_handler![enumerate_devices, set_acrylic])
        .run(tauri::generate_context!())
        .expect("error while running DrvMatch");
}
