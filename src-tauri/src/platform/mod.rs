#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::enumerate_devices;

#[cfg(not(windows))]
pub fn enumerate_devices() -> Result<Vec<crate::domain::Device>, String> {
    Err("Device inventory is available only on Windows".to_string())
}
