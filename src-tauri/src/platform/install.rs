use std::{
    ffi::OsStr,
    fs::{self, File},
    io::Read,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use windows::{
    Win32::{
        Devices::DeviceAndDriverInstallation::{
            DIINSTALLDRIVER_FLAGS, DIROLLBACKDRIVER_FLAGS, DiInstallDriverW, DiRollbackDriver,
            SP_DEVINFO_DATA, SetupDiCreateDeviceInfoList, SetupDiDestroyDeviceInfoList,
            SetupDiOpenDeviceInfoW,
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
                BEGIN_SYSTEM_CHANGE, DEVICE_DRIVER_INSTALL, END_SYSTEM_CHANGE, RESTOREPOINTINFOW,
                STATEMGRSTATUS,
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
    /// Exact artifact downloaded and hashed by the unelevated process. The elevated
    /// helper hashes it again before doing any system work so a package changed
    /// between review and UAC approval is rejected.
    pub source_package_path: PathBuf,
    pub expected_sha256: String,
    /// Hash of the exact verified INF/vendor installer selected for execution.
    /// For direct INF/EXE/MSI packages this equals `expected_sha256`; for CAB
    /// packages it pins the selected extracted INF against a second TOCTOU.
    pub expected_install_sha256: String,
    /// Exact verified INF or vendor installer that Windows should execute. CAB
    /// containers are expanded and narrowed to one target INF before elevation.
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
    let (mode, request_path, result_path, expected_request_sha256) = match args {
        [_, mode, request, result, request_sha256]
            if mode == "--drvmatch-install-helper" || mode == "--drvmatch-rollback-helper" =>
        {
            (mode, request, result, request_sha256)
        }
        _ => return None,
    };

    // The request lives in the unelevated user's app-data directory. Pin the
    // exact serialized request in the already-created elevated process command
    // line so another process cannot race-edit both paths/flags and their hashes
    // between the UAC review and privileged execution.
    let request_bytes = fs::read(request_path)
        .map_err(|error| format!("Could not read the protected operation request: {error}"));
    let result = request_bytes
        .and_then(|bytes| {
            let actual = sha256_bytes(&bytes);
            if !actual.eq_ignore_ascii_case(expected_request_sha256) {
                return Err(
                    "The privileged operation request changed after UAC review; the operation was blocked."
                        .into(),
                );
            }
            Ok(bytes)
        })
        .and_then(|bytes| {
            if mode == "--drvmatch-install-helper" {
                serde_json::from_slice::<ElevatedInstallRequest>(&bytes)
                    .map_err(|error| format!("The protected install request is invalid: {error}"))
                    .and_then(perform_install)
            } else {
                serde_json::from_slice::<ElevatedRollbackRequest>(&bytes)
                    .map_err(|error| format!("The protected rollback request is invalid: {error}"))
                    .and_then(perform_rollback)
            }
        });
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
    let _ = fs::remove_file(&request_path);
    let _ = fs::remove_file(&result_path);
    let request_bytes = serde_json::to_vec(request)
        .map_err(|error| format!("Could not serialize the privileged request: {error}"))?;
    let request_sha256 = sha256_bytes(&request_bytes);
    fs::write(&request_path, &request_bytes)
        .map_err(|error| format!("Could not write the privileged request: {error}"))?;
    let executable =
        std::env::current_exe().map_err(|error| format!("Could not locate DrvMatch: {error}"))?;
    let parameters = format!(
        "{mode} \"{}\" \"{}\" {request_sha256}",
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
    let parsed = fs::read_to_string(&result_path)
        .map_err(|error| {
            format!("The elevated installer did not return a result (exit {exit_code}): {error}")
        })
        .and_then(|payload| {
            serde_json::from_str(&payload).map_err(|error| {
                format!("The elevated installer returned an invalid result: {error}")
            })
        });
    let _ = fs::remove_file(request_path);
    let _ = fs::remove_file(result_path);
    parsed
}

fn perform_install(request: ElevatedInstallRequest) -> Result<ElevatedInstallResult, String> {
    if request.device_instance_id.trim().is_empty() {
        return Err("The protected install request did not identify a device.".into());
    }
    verify_sha256(&request.source_package_path, &request.expected_sha256)?;
    verify_sha256(&request.package_path, &request.expected_install_sha256)?;
    match request.package_kind.as_str() {
        "vendor-installer" => verify_file_signature(&request.package_path)?,
        "inf" => {
            if !request.package_path.is_file() {
                return Err(
                    "DrvMatch only elevates a single verified INF package at a time.".into(),
                );
            }
            verify_inf_signature(&request.package_path)?;
        }
        _ => return Err("The protected install request used an unsupported package type.".into()),
    }

    let mut result = ElevatedInstallResult::default();

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
        if status.success()
            && find_inf_files(&request.backup_directory)
                .map(|infs| !infs.is_empty())
                .unwrap_or(false)
        {
            result.backup_path = Some(request.backup_directory.display().to_string());
        }
    }

    let restore_sequence = if request.create_restore_point {
        result.restore_point_attempted = true;
        begin_restore_point()
    } else {
        None
    };

    let install_result = if request.package_kind == "vendor-installer" {
        run_vendor_installer(&request.package_path)
    } else {
        install_inf(&request.package_path)
    };

    if let Some(sequence) = restore_sequence {
        result.restore_point_created = end_restore_point(sequence);
    }

    match install_result {
        Ok((reboot_required, message)) => {
            result.succeeded = true;
            result.reboot_required = reboot_required;
            result.message = message;
        }
        Err(message) => {
            result.succeeded = false;
            result.message = message;
        }
    }

    Ok(result)
}

fn run_vendor_installer(path: &Path) -> Result<(bool, String), String> {
    let status = if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("msi"))
    {
        Command::new(system_tool("msiexec.exe"))
            .arg("/i")
            .arg(path)
            .status()
    } else {
        Command::new(path).status()
    }
    .map_err(|error| format!("Could not start the verified vendor installer: {error}"))?;

    if status.success() || status.code() == Some(3010) {
        Ok((
            status.code() == Some(3010),
            "The verified vendor installer completed.".into(),
        ))
    } else {
        Err(format!(
            "The vendor installer exited with code {:?}.",
            status.code()
        ))
    }
}

fn install_inf(path: &Path) -> Result<(bool, String), String> {
    let mut reboot = BOOL(0);
    let path = wide(path.as_os_str());
    unsafe {
        DiInstallDriverW(
            None,
            PCWSTR(path.as_ptr()),
            DIINSTALLDRIVER_FLAGS(0),
            Some(&mut reboot),
        )
    }
    .map_err(|error| format!("Windows rejected the driver package: {error}"))?;
    Ok((
        reboot.as_bool(),
        "Windows accepted the signed INF package without forcing a lower-ranked match.".into(),
    ))
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
    rolled_back.map_err(|error| {
        format!("Windows could not use its native backup driver for this device: {error}")
    })?;
    Ok(ElevatedInstallResult {
        succeeded: true,
        message: "Windows rolled the device back to its native backup driver.".into(),
        reboot_required: reboot.as_bool(),
        ..Default::default()
    })
}

fn begin_restore_point() -> Option<i64> {
    let function = load_restore_point_function()?;
    let request = RESTOREPOINTINFOW {
        dwEventType: BEGIN_SYSTEM_CHANGE,
        dwRestorePtType: DEVICE_DRIVER_INSTALL,
        llSequenceNumber: 0,
        szDescription: restore_point_description(),
    };
    let mut status = STATEMGRSTATUS::default();
    let created = unsafe { (function.1)(&request, &mut status) }.as_bool() && status.nStatus.0 == 0;
    unsafe {
        let _ = FreeLibrary(function.0);
    }
    created.then_some(status.llSequenceNumber)
}

fn end_restore_point(sequence: i64) -> bool {
    let Some((module, function)) = load_restore_point_function() else {
        return false;
    };
    let request = RESTOREPOINTINFOW {
        dwEventType: END_SYSTEM_CHANGE,
        dwRestorePtType: DEVICE_DRIVER_INSTALL,
        llSequenceNumber: sequence,
        szDescription: restore_point_description(),
    };
    let mut status = STATEMGRSTATUS::default();
    let ended = unsafe { function(&request, &mut status) }.as_bool() && status.nStatus.0 == 0;
    unsafe {
        let _ = FreeLibrary(module);
    }
    ended
}

type SetRestorePoint =
    unsafe extern "system" fn(*const RESTOREPOINTINFOW, *mut STATEMGRSTATUS) -> BOOL;

fn load_restore_point_function() -> Option<(windows::Win32::Foundation::HMODULE, SetRestorePoint)> {
    let module = unsafe { LoadLibraryW(w!("SrClient.dll")) }.ok()?;
    let address = unsafe { GetProcAddress(module, s!("SRSetRestorePointW")) };
    let Some(address) = address else {
        unsafe {
            let _ = FreeLibrary(module);
        }
        return None;
    };
    let function: SetRestorePoint = unsafe { std::mem::transmute(address) };
    Some((module, function))
}

fn restore_point_description() -> [u16; 256] {
    let mut description = [0u16; 256];
    for (target, source) in description
        .iter_mut()
        .zip("DrvMatch driver installation".encode_utf16())
    {
        *target = source;
    }
    description
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let mut file = File::open(path)
        .map_err(|error| format!("Could not reopen the reviewed package for hashing: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not re-hash the reviewed package: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected.trim()) {
        return Err(
            "The reviewed package changed before elevation; installation was blocked.".into(),
        );
    }
    Ok(())
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
    result.sort();
    Ok(result)
}

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
