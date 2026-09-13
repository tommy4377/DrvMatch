use std::{
    ffi::OsStr,
    fs,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use windows::{
    Win32::{
        Devices::DeviceAndDriverInstallation::{
            DIINSTALLDRIVER_FLAGS, DIIRFLAG_INSTALL_AS_SET, DIROLLBACKDRIVER_FLAGS,
            DiInstallDriverW, DiRollbackDriver, SP_DEVINFO_DATA, SetupDiCreateDeviceInfoList,
            SetupDiDestroyDeviceInfoList, SetupDiOpenDeviceInfoW,
        },
        Foundation::{CloseHandle, FreeLibrary, HANDLE, HWND},
        Security::WinTrust::{
            WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_FILE_INFO,
            WTD_CHOICE_FILE, WTD_REVOKE_WHOLECHAIN, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY,
            WTD_UI_NONE, WinVerifyTrust,
        },
        System::{
            LibraryLoader::{GetProcAddress, LoadLibraryW},
            Restore::{
                BEGIN_SYSTEM_CHANGE, DEVICE_DRIVER_INSTALL, RESTOREPOINTINFOW, STATEMGRSTATUS,
            },
            Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject},
        },
        UI::{
            Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
    core::{BOOL, PCWSTR, s, w},
};
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SP_INF_SIGNER_INFO_V2_W, SetupVerifyInfFileW,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevatedInstallRequest {
    pub package_path: PathBuf,
    pub package_kind: String,
    pub device_instance_id: String,
    pub current_inf: Option<String>,
    pub backup_directory: PathBuf,
    pub create_restore_point: bool,
    pub backup_current_package: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevatedInstallResult {
    pub succeeded: bool,
    pub message: String,
    pub reboot_required: bool,
    pub restore_point_attempted: bool,
    pub restore_point_created: bool,
    pub backup_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevatedRollbackRequest {
    pub device_instance_id: String,
}

pub fn run_privileged_helper(args: &[String]) -> Option<i32> {
    let (mode, request_path, result_path) = match args {
        [_, mode, request, result]
            if mode == "--drvmatch-install-helper" || mode == "--drvmatch-rollback-helper" =>
        {
            (mode, request, result)
        }
        _ => return None,
    };
    let result = if mode == "--drvmatch-install-helper" {
        fs::read_to_string(request_path)
            .map_err(|error| format!("Could not read the protected install request: {error}"))
            .and_then(|json| {
                serde_json::from_str::<ElevatedInstallRequest>(&json)
                    .map_err(|error| format!("The protected install request is invalid: {error}"))
            })
            .and_then(perform_install)
    } else {
        fs::read_to_string(request_path)
            .map_err(|error| format!("Could not read the protected rollback request: {error}"))
            .and_then(|json| {
                serde_json::from_str::<ElevatedRollbackRequest>(&json)
                    .map_err(|error| format!("The protected rollback request is invalid: {error}"))
            })
            .and_then(perform_rollback)
    };
    let payload = match result {
        Ok(result) => result,
        Err(message) => ElevatedInstallResult {
            message,
            ..Default::default()
        },
    };
    let exit = if payload.succeeded { 0 } else { 1 };
    let _ = fs::write(
        result_path,
        serde_json::to_vec(&payload).unwrap_or_default(),
    );
    Some(exit)
}

pub fn run_elevated_install(
    request: &ElevatedInstallRequest,
    exchange_directory: &Path,
) -> Result<ElevatedInstallResult, String> {
    run_elevated("--drvmatch-install-helper", request, exchange_directory)
}

pub fn run_elevated_rollback(
    request: &ElevatedRollbackRequest,
    exchange_directory: &Path,
) -> Result<ElevatedInstallResult, String> {
    run_elevated("--drvmatch-rollback-helper", request, exchange_directory)
}

fn run_elevated<T: Serialize>(
    mode: &str,
    request: &T,
    exchange_directory: &Path,
) -> Result<ElevatedInstallResult, String> {
    fs::create_dir_all(exchange_directory)
        .map_err(|error| format!("Could not create the protected operation directory: {error}"))?;
    let request_path = exchange_directory.join("request.json");
    let result_path = exchange_directory.join("result.json");
    fs::write(
        &request_path,
        serde_json::to_vec(request)
            .map_err(|error| format!("Could not serialize the privileged request: {error}"))?,
    )
    .map_err(|error| format!("Could not write the privileged request: {error}"))?;
    let executable =
        std::env::current_exe().map_err(|error| format!("Could not locate DrvMatch: {error}"))?;
    let parameters = format!(
        "{mode} \"{}\" \"{}\"",
        request_path.display(),
        result_path.display()
    );
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    let verb = wide("runas");
    let executable = wide(executable.as_os_str());
    let parameters = wide(&parameters);
    info.lpVerb = PCWSTR(verb.as_ptr());
    info.lpFile = PCWSTR(executable.as_ptr());
    info.lpParameters = PCWSTR(parameters.as_ptr());
    unsafe { ShellExecuteExW(&mut info) }
        .map_err(|error| format!("Administrator approval was not granted: {error}"))?;
    if info.hProcess == HANDLE::default() {
        return Err("Windows did not return the elevated installer process.".into());
    }
    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
    }
    let mut exit_code = 1;
    let _ = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    let _ = unsafe { CloseHandle(info.hProcess) };
    let payload = fs::read_to_string(&result_path).map_err(|error| {
        format!("The elevated installer did not return a result (exit {exit_code}): {error}")
    })?;
    serde_json::from_str(&payload)
        .map_err(|error| format!("The elevated installer returned an invalid result: {error}"))
}

fn perform_install(request: ElevatedInstallRequest) -> Result<ElevatedInstallResult, String> {
    if request.package_kind == "vendor-installer" {
        verify_file_signature(&request.package_path)?;
    } else if request.package_path.is_dir() {
        let infs = find_inf_files(&request.package_path)?;
        if infs.is_empty() {
            return Err("The staged package contains no INF files.".into());
        }
        for inf in infs {
            verify_inf_signature(&inf)?;
        }
    } else {
        verify_inf_signature(&request.package_path)?;
    }
    let mut result = ElevatedInstallResult::default();
    if request.create_restore_point {
        result.restore_point_attempted = true;
        result.restore_point_created = create_restore_point();
    }
    if request.backup_current_package
        && let Some(inf) = request.current_inf.as_deref()
    {
        fs::create_dir_all(&request.backup_directory)
            .map_err(|error| format!("Could not create the driver backup directory: {error}"))?;
        let status = Command::new(system_tool("pnputil.exe"))
            .args(["/export-driver", inf])
            .arg(&request.backup_directory)
            .status()
            .map_err(|error| format!("Could not start Windows driver export: {error}"))?;
        if status.success() {
            result.backup_path = Some(request.backup_directory.display().to_string());
        }
    }
    if request.package_kind == "vendor-installer" {
        let status = if request
            .package_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("msi"))
        {
            Command::new(system_tool("msiexec.exe"))
                .arg("/i")
                .arg(&request.package_path)
                .status()
        } else {
            Command::new(&request.package_path).status()
        }
        .map_err(|error| format!("Could not start the verified vendor installer: {error}"))?;
        result.succeeded = status.success() || status.code() == Some(3010);
        result.reboot_required = status.code() == Some(3010);
        result.message = if result.succeeded {
            "The verified vendor installer completed.".into()
        } else {
            format!("The vendor installer exited with code {:?}.", status.code())
        };
        return Ok(result);
    }
    let mut reboot = BOOL(0);
    let path = wide(request.package_path.as_os_str());
    let flags = if request.package_path.is_dir() {
        DIIRFLAG_INSTALL_AS_SET
    } else {
        DIINSTALLDRIVER_FLAGS(0)
    };
    unsafe { DiInstallDriverW(None, PCWSTR(path.as_ptr()), flags, Some(&mut reboot)) }
        .map_err(|error| format!("Windows rejected the driver package: {error}"))?;
    result.succeeded = true;
    result.reboot_required = reboot.as_bool();
    result.message =
        "Windows installed the signed driver package without forcing a lower-ranked match.".into();
    Ok(result)
}

fn perform_rollback(request: ElevatedRollbackRequest) -> Result<ElevatedInstallResult, String> {
    let device_set = unsafe { SetupDiCreateDeviceInfoList(None, None) }
        .map_err(|error| format!("Could not create a Windows device set: {error}"))?;
    let mut info = SP_DEVINFO_DATA {
        cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };
    let instance_id = wide(&request.device_instance_id);
    let opened = unsafe {
        SetupDiOpenDeviceInfoW(
            device_set,
            PCWSTR(instance_id.as_ptr()),
            None,
            0,
            Some(&mut info),
        )
    };
    if let Err(error) = opened {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(device_set);
        }
        return Err(format!(
            "The device is no longer available for rollback: {error}"
        ));
    }
    let mut reboot = BOOL(0);
    let rolled_back = unsafe {
        DiRollbackDriver(
            device_set,
            &info,
            None,
            DIROLLBACKDRIVER_FLAGS(0),
            Some(&mut reboot),
        )
    };
    unsafe {
        let _ = SetupDiDestroyDeviceInfoList(device_set);
    }
    rolled_back.map_err(|error| format!("Windows could not roll back this device: {error}"))?;
    Ok(ElevatedInstallResult {
        succeeded: true,
        message: "Windows rolled the device back to its previous driver.".into(),
        reboot_required: reboot.as_bool(),
        ..Default::default()
    })
}

fn create_restore_point() -> bool {
    let mut description = [0u16; 256];
    for (target, source) in description
        .iter_mut()
        .zip("DrvMatch driver installation".encode_utf16())
    {
        *target = source;
    }
    let request = RESTOREPOINTINFOW {
        dwEventType: BEGIN_SYSTEM_CHANGE,
        dwRestorePtType: DEVICE_DRIVER_INSTALL,
        llSequenceNumber: 0,
        szDescription: description,
    };
    let mut status = STATEMGRSTATUS::default();
    let Ok(module) = (unsafe { LoadLibraryW(w!("SrClient.dll")) }) else {
        return false;
    };
    let address = unsafe { GetProcAddress(module, s!("SRSetRestorePointW")) };
    let Some(address) = address else {
        unsafe {
            let _ = FreeLibrary(module);
        }
        return false;
    };
    type SetRestorePoint =
        unsafe extern "system" fn(*const RESTOREPOINTINFOW, *mut STATEMGRSTATUS) -> BOOL;
    let function: SetRestorePoint = unsafe { std::mem::transmute(address) };
    let created = unsafe { function(&request, &mut status) }.as_bool() && status.nStatus.0 == 0;
    unsafe {
        let _ = FreeLibrary(module);
    }
    created
}

pub fn verify_inf_signature(path: &Path) -> Result<(), String> {
    let path = wide(path.as_os_str());
    let mut info = SP_INF_SIGNER_INFO_V2_W {
        cbSize: size_of::<SP_INF_SIGNER_INFO_V2_W>() as u32,
        ..Default::default()
    };
    if unsafe { SetupVerifyInfFileW(path.as_ptr(), std::ptr::null(), &mut info) } == 0 {
        return Err(
            "The INF package does not have a valid Windows-trusted catalog signature.".into(),
        );
    }
    Ok(())
}

pub fn verify_file_signature(path: &Path) -> Result<(), String> {
    let path = wide(path.as_os_str());
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(path.as_ptr()),
        ..Default::default()
    };
    let mut data = WINTRUST_DATA {
        cbStruct: size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &mut file },
        dwStateAction: WTD_STATEACTION_VERIFY,
        ..Default::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let status =
        unsafe { WinVerifyTrust(HWND::default(), &mut action, &mut data as *mut _ as *mut _) };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    let _ = unsafe { WinVerifyTrust(HWND::default(), &mut action, &mut data as *mut _ as *mut _) };
    if status != 0 {
        return Err(format!(
            "Windows did not trust the downloaded package signature (status 0x{:08X}).",
            status as u32
        ));
    }
    Ok(())
}

fn system_tool(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into()))
        .join("System32")
        .join(name)
}

fn find_inf_files(root: &Path) -> Result<Vec<PathBuf>, String> {
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
                .is_some_and(|value| value.eq_ignore_ascii_case("inf"))
            {
                result.push(path);
            }
        }
    }
    Ok(result)
}

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
