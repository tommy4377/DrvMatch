#[cfg(windows)]
mod install;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{enumerate_devices, windows_accent_color};

#[cfg(windows)]
pub use install::{
    ElevatedInstallRequest, ElevatedInstallResult, ElevatedRollbackRequest, run_elevated_install,
    run_elevated_rollback, run_privileged_helper, verify_file_signature, verify_inf_signature,
};

#[cfg(not(windows))]
pub fn enumerate_devices() -> Result<Vec<crate::domain::Device>, String> {
    Err("Device inventory is available only on Windows".to_string())
}
