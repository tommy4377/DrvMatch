use std::{
    collections::HashMap,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};

use windows_sys::Win32::Devices::Properties::{
    DEVPKEY_Device_Driver, DEVPKEY_Device_DriverDate, DEVPKEY_Device_DriverDesc,
    DEVPKEY_Device_DriverInfPath, DEVPKEY_Device_DriverInfSection, DEVPKEY_Device_DriverLogoLevel,
    DEVPKEY_Device_DriverProvider, DEVPKEY_Device_DriverRank, DEVPKEY_Device_DriverVersion,
    DEVPKEY_Device_MatchingDeviceId, DEVPKEY_Device_ProblemCode, DEVPKEY_Device_ProblemStatus,
    DEVPROP_TYPE_FILETIME, DEVPROP_TYPE_INT32, DEVPROP_TYPE_NTSTATUS, DEVPROP_TYPE_STRING,
    DEVPROP_TYPE_STRING_INDIRECT, DEVPROP_TYPE_UINT32,
};
use windows_sys::Win32::{
    Devices::DeviceAndDriverInstallation::{
        DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SIGNERSCORE_AUTHENTICODE, SIGNERSCORE_INBOX,
        SIGNERSCORE_LOGO_PREMIUM, SIGNERSCORE_LOGO_STANDARD, SIGNERSCORE_UNCLASSIFIED,
        SIGNERSCORE_UNKNOWN, SIGNERSCORE_UNSIGNED, SIGNERSCORE_WHQL, SP_DEVINFO_DATA,
        SP_INF_SIGNER_INFO_V2_W, SPDRP_CLASS, SPDRP_CLASSGUID, SPDRP_COMPATIBLEIDS,
        SPDRP_DEVICEDESC, SPDRP_FRIENDLYNAME, SPDRP_HARDWAREID, SPDRP_MFG,
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        SetupDiGetDeviceInstanceIdW, SetupDiGetDevicePropertyW, SetupDiGetDeviceRegistryPropertyW,
        SetupVerifyInfFileW,
    },
    Foundation::{DEVPROPKEY, ERROR_NO_MORE_ITEMS, GetLastError, INVALID_HANDLE_VALUE},
    Graphics::Dwm::{
        DwmGetColorizationColor, DwmSetWindowAttribute, DWMWA_BORDER_COLOR,
        DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    },
};

use windows_registry::LOCAL_MACHINE;

use crate::domain::{
    Device, DeviceCondition, HardwareIdentity, InstalledDriver, MachineIdentity, SignatureStatus,
};

const PROPERTY_BUFFER_BYTES: usize = 32 * 1024;
const INSTANCE_ID_BUFFER_CHARS: usize = 4096;

struct DeviceInfoSet(HDEVINFO);

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        unsafe { SetupDiDestroyDeviceInfoList(self.0) };
    }
}

pub fn windows_accent_color() -> Option<String> {
    let mut color = 0u32;
    let mut opaque = 0;
    let result = unsafe { DwmGetColorizationColor(&mut color, &mut opaque) };
    (result == 0).then(|| format!("#{:06X}", color & 0x00ff_ffff))
}

/// Applies the small amount of native chrome DrvMatch still wants on its
/// undecorated Windows 11 window. Tauri's `shadow: true` intentionally asks
/// Windows for a native shadow, but on an undecorated window that also creates
/// a visible one-pixel frame. DrvMatch paints the full client area itself, so
/// suppress that DWM border and ask Windows only for native corner clipping.
pub fn configure_main_window_chrome(hwnd: isize) {
    use std::{ffi::c_void, mem::size_of_val};

    let hwnd = hwnd as windows_sys::Win32::Foundation::HWND;
    let border = DWMWA_COLOR_NONE;
    let corner = DWMWCP_ROUND;

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            &border as *const _ as *const c_void,
            size_of_val(&border) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &corner as *const _ as *const c_void,
            size_of_val(&corner) as u32,
        );
    }
}

pub fn detect_machine_identity() -> MachineIdentity {
    let bios = LOCAL_MACHINE
        .open("HARDWARE\\DESCRIPTION\\System\\BIOS")
        .ok();
    let current_version = LOCAL_MACHINE
        .open("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .ok();
    let read_bios = |name: &str| {
        bios.as_ref()
            .and_then(|key| key.get_string(name).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty() && !is_placeholder(value))
    };
    let read_windows = |name: &str| {
        current_version
            .as_ref()
            .and_then(|key| key.get_string(name).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    };
    MachineIdentity {
        manufacturer: read_bios("SystemManufacturer"),
        model: read_bios("SystemProductName"),
        system_sku: read_bios("SystemSKU"),
        system_family: read_bios("SystemFamily"),
        baseboard_manufacturer: read_bios("BaseBoardManufacturer"),
        baseboard_product: read_bios("BaseBoardProduct"),
        bios_version: read_bios("BIOSVersion"),
        windows_display_version: read_windows("DisplayVersion")
            .or_else(|| read_windows("ReleaseId")),
        windows_build: read_windows("CurrentBuildNumber").or_else(|| read_windows("CurrentBuild")),
    }
}

pub fn enumerate_devices() -> Result<Vec<Device>, String> {
    let raw_set = unsafe {
        SetupDiGetClassDevsW(null(), null(), null_mut(), DIGCF_ALLCLASSES | DIGCF_PRESENT)
    };
    if raw_set == INVALID_HANDLE_VALUE as isize {
        return Err(last_error("Windows could not open the present-device set"));
    }
    let device_set = DeviceInfoSet(raw_set);
    let mut devices = Vec::new();
    let mut signature_cache = HashMap::new();
    let mut index = 0;

    loop {
        let mut info: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
        info.cbSize = size_of::<SP_DEVINFO_DATA>() as u32;
        if unsafe { SetupDiEnumDeviceInfo(device_set.0, index, &mut info) } == 0 {
            let error = unsafe { GetLastError() };
            if error == ERROR_NO_MORE_ITEMS {
                break;
            }
            return Err(format!(
                "Windows stopped device enumeration (error {error})"
            ));
        }

        let instance_id = read_instance_id(device_set.0, &mut info)
            .unwrap_or_else(|| format!("unknown-device-{index}"));
        let description = read_string_property(device_set.0, &mut info, SPDRP_DEVICEDESC)
            .unwrap_or_else(|| "Unknown device".to_string());
        let mut friendly_name = read_string_property(device_set.0, &mut info, SPDRP_FRIENDLYNAME)
            .unwrap_or_else(|| description.clone());

        let hardware_ids = read_multi_string_property(device_set.0, &mut info, SPDRP_HARDWAREID);
        let hardware_identity = identify_hardware(&hardware_ids);
        if friendly_name.eq_ignore_ascii_case("unknown device")
            && let Some(identity) = &hardware_identity
        {
            friendly_name = identity.description.clone();
        }
        let problem_code =
            read_u32_device_property(device_set.0, &mut info, &DEVPKEY_Device_ProblemCode)
                .filter(|code| *code != 0);
        let problem_status =
            read_i32_device_property(device_set.0, &mut info, &DEVPKEY_Device_ProblemStatus)
                .filter(|status| *status != 0);
        let installed_driver = read_installed_driver(
            device_set.0,
            &mut info,
            &friendly_name,
            &mut signature_cache,
        );
        let condition = device_condition(
            problem_code,
            problem_status,
            installed_driver.is_some(),
            !hardware_ids.is_empty(),
        );

        devices.push(Device {
            instance_id,
            friendly_name,
            description,
            manufacturer: read_string_property(device_set.0, &mut info, SPDRP_MFG),
            class_name: read_string_property(device_set.0, &mut info, SPDRP_CLASS),
            class_guid: read_string_property(device_set.0, &mut info, SPDRP_CLASSGUID),
            hardware_ids,
            compatible_ids: read_multi_string_property(
                device_set.0,
                &mut info,
                SPDRP_COMPATIBLEIDS,
            ),
            present: true,
            problem_code,
            problem_status,
            condition,
            installed_driver,
            hardware_identity,
        });
        index += 1;
    }

    devices.sort_by(|left, right| {
        left.class_name
            .cmp(&right.class_name)
            .then_with(|| left.friendly_name.cmp(&right.friendly_name))
            .then_with(|| left.instance_id.cmp(&right.instance_id))
    });
    Ok(devices)
}

fn identify_hardware(ids: &[String]) -> Option<HardwareIdentity> {
    let id = ids.first()?.to_ascii_uppercase();
    let (bus, vendor_marker, device_marker) = if id.starts_with("PCI\\") {
        ("PCI", "VEN_", "DEV_")
    } else if id.starts_with("USB\\") {
        ("USB", "VID_", "PID_")
    } else if id.starts_with("HDAUDIO\\") {
        ("HD Audio", "VEN_", "DEV_")
    } else if id.starts_with("ACPI\\") {
        ("ACPI", "VEN_", "DEV_")
    } else {
        return Some(HardwareIdentity {
            bus: id.split('\\').next().unwrap_or("Device").to_string(),
            description: format!("Unknown device ({})", id.split('\\').next().unwrap_or("ID")),
            ..HardwareIdentity::default()
        });
    };
    let vendor_id = id_component(&id, vendor_marker);
    let device_id = id_component(&id, device_marker);
    let subsystem_id = id_component(&id, "SUBSYS_");
    let identity = [
        vendor_id
            .as_deref()
            .map(|value| format!("{vendor_marker}{value}")),
        device_id
            .as_deref()
            .map(|value| format!("{device_marker}{value}")),
        subsystem_id
            .as_deref()
            .map(|value| format!("SUBSYS_{value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    Some(HardwareIdentity {
        bus: bus.into(),
        vendor_id,
        device_id,
        subsystem_id,
        description: if identity.is_empty() {
            format!("Unknown {bus} device")
        } else {
            format!("Unknown {bus} device ({identity})")
        },
    })
}

fn id_component(id: &str, marker: &str) -> Option<String> {
    let start = id.find(marker)? + marker.len();
    let value = id[start..]
        .split(['&', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn is_placeholder(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "to be filled by o.e.m." | "system product name" | "default string" | "not applicable"
    )
}

fn device_condition(
    problem_code: Option<u32>,
    problem_status: Option<i32>,
    has_installed_driver: bool,
    has_hardware_ids: bool,
) -> DeviceCondition {
    if problem_code == Some(28) || (!has_installed_driver && has_hardware_ids) {
        DeviceCondition::Missing
    } else if problem_code.is_some() || problem_status.is_some() {
        DeviceCondition::Problem
    } else {
        DeviceCondition::Current
    }
}

#[derive(Clone, Debug, Default)]
struct SignatureDetails {
    signer: Option<String>,
    catalog_file: Option<String>,
    verified: bool,
}

fn read_installed_driver(
    device_set: HDEVINFO,
    info: &mut SP_DEVINFO_DATA,
    friendly_name: &str,
    signature_cache: &mut HashMap<String, SignatureDetails>,
) -> Option<InstalledDriver> {
    let description = read_string_device_property(device_set, info, &DEVPKEY_Device_DriverDesc);
    let provider = read_string_device_property(device_set, info, &DEVPKEY_Device_DriverProvider);
    let version = read_string_device_property(device_set, info, &DEVPKEY_Device_DriverVersion);
    let published_inf_name =
        read_string_device_property(device_set, info, &DEVPKEY_Device_DriverInfPath);
    let matching_id =
        read_string_device_property(device_set, info, &DEVPKEY_Device_MatchingDeviceId);
    let driver_key = read_string_device_property(device_set, info, &DEVPKEY_Device_Driver);
    let inf_section =
        read_string_device_property(device_set, info, &DEVPKEY_Device_DriverInfSection);
    let driver_date = read_filetime_device_property(device_set, info, &DEVPKEY_Device_DriverDate);
    let driver_rank = read_u32_device_property(device_set, info, &DEVPKEY_Device_DriverRank);
    let signer_score = read_u32_device_property(device_set, info, &DEVPKEY_Device_DriverLogoLevel);

    if description.is_none()
        && provider.is_none()
        && version.is_none()
        && published_inf_name.is_none()
    {
        return None;
    }

    let inf_path = published_inf_name.as_deref().map(installed_inf_path);
    let signature_details = inf_path
        .as_ref()
        .map(|path| {
            let key = path.to_string_lossy().to_string();
            signature_cache
                .entry(key)
                .or_insert_with(|| verify_inf(path))
                .clone()
        })
        .unwrap_or_default();
    let signature = signature_status(signer_score);
    let generic_microsoft =
        is_explicit_generic_microsoft(provider.as_deref(), description.as_deref(), friendly_name);

    Some(InstalledDriver {
        description,
        provider,
        version,
        driver_date,
        inf_path: inf_path.map(|path| path.to_string_lossy().to_string()),
        published_inf_name,
        inf_section,
        matching_id,
        driver_key,
        driver_rank,
        signer: signature_details.signer,
        catalog_file: signature_details.catalog_file,
        signature,
        inf_signature_verified: signature_details.verified,
        generic_microsoft,
    })
}

fn is_explicit_generic_microsoft(
    provider: Option<&str>,
    driver_description: Option<&str>,
    friendly_name: &str,
) -> bool {
    provider.is_some_and(|value| value.eq_ignore_ascii_case("Microsoft"))
        && driver_description
            .unwrap_or(friendly_name)
            .to_ascii_lowercase()
            .contains("generic")
}

fn installed_inf_path(inf_name: &str) -> PathBuf {
    let path = PathBuf::from(inf_name);
    if path.is_absolute() {
        path
    } else {
        PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into()))
            .join("INF")
            .join(path)
    }
}

fn verify_inf(path: &Path) -> SignatureDetails {
    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);
    let mut info = SP_INF_SIGNER_INFO_V2_W {
        cbSize: size_of::<SP_INF_SIGNER_INFO_V2_W>() as u32,
        ..Default::default()
    };
    let verified = unsafe { SetupVerifyInfFileW(wide_path.as_ptr(), null(), &mut info) } != 0;
    SignatureDetails {
        signer: non_empty_utf16(&info.DigitalSigner),
        catalog_file: non_empty_utf16(&info.CatalogFile),
        verified,
    }
}

fn signature_status(score: Option<u32>) -> SignatureStatus {
    match score {
        Some(SIGNERSCORE_LOGO_PREMIUM | SIGNERSCORE_LOGO_STANDARD | SIGNERSCORE_WHQL) => {
            SignatureStatus::Whql
        }
        Some(SIGNERSCORE_INBOX) => SignatureStatus::Inbox,
        Some(SIGNERSCORE_AUTHENTICODE) => SignatureStatus::Authenticode,
        Some(SIGNERSCORE_UNCLASSIFIED) => SignatureStatus::SignedUnclassified,
        Some(SIGNERSCORE_UNSIGNED) => SignatureStatus::Unsigned,
        Some(SIGNERSCORE_UNKNOWN) | None => SignatureStatus::Unknown,
        Some(_) => SignatureStatus::Unknown,
    }
}

fn read_device_property(
    device_set: HDEVINFO,
    info: &mut SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<(u32, Vec<u8>)> {
    let mut buffer = vec![0u8; PROPERTY_BUFFER_BYTES];
    let mut property_type = 0;
    let mut required = 0;
    let success = unsafe {
        SetupDiGetDevicePropertyW(
            device_set,
            info,
            key,
            &mut property_type,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            &mut required,
            0,
        )
    };
    if success == 0 || required == 0 {
        return None;
    }
    buffer.truncate((required as usize).min(buffer.len()));
    Some((property_type, buffer))
}

fn read_string_device_property(
    device_set: HDEVINFO,
    info: &mut SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<String> {
    let (property_type, bytes) = read_device_property(device_set, info, key)?;
    if !matches!(
        property_type,
        DEVPROP_TYPE_STRING | DEVPROP_TYPE_STRING_INDIRECT
    ) {
        return None;
    }
    non_empty_utf16_bytes(&bytes)
}

fn read_u32_device_property(
    device_set: HDEVINFO,
    info: &mut SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<u32> {
    let (property_type, bytes) = read_device_property(device_set, info, key)?;
    if property_type != DEVPROP_TYPE_UINT32 {
        return None;
    }
    let value: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
    Some(u32::from_le_bytes(value))
}

fn read_i32_device_property(
    device_set: HDEVINFO,
    info: &mut SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<i32> {
    let (property_type, bytes) = read_device_property(device_set, info, key)?;
    if !matches!(property_type, DEVPROP_TYPE_INT32 | DEVPROP_TYPE_NTSTATUS) {
        return None;
    }
    let value: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
    Some(i32::from_le_bytes(value))
}

fn read_filetime_device_property(
    device_set: HDEVINFO,
    info: &mut SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<i64> {
    const WINDOWS_TO_UNIX_TICKS: u64 = 116_444_736_000_000_000;
    const TICKS_PER_SECOND: u64 = 10_000_000;
    let (property_type, bytes) = read_device_property(device_set, info, key)?;
    if property_type != DEVPROP_TYPE_FILETIME {
        return None;
    }
    let value: [u8; 8] = bytes.get(..8)?.try_into().ok()?;
    let ticks = u64::from_le_bytes(value);
    (ticks >= WINDOWS_TO_UNIX_TICKS)
        .then(|| ((ticks - WINDOWS_TO_UNIX_TICKS) / TICKS_PER_SECOND) as i64)
}

fn non_empty_utf16(value: &[u16]) -> Option<String> {
    let value = decode_utf16(value);
    (!value.is_empty()).then_some(value)
}

fn non_empty_utf16_bytes(bytes: &[u8]) -> Option<String> {
    let value: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    non_empty_utf16(&value)
}

fn read_instance_id(device_set: HDEVINFO, info: &mut SP_DEVINFO_DATA) -> Option<String> {
    let mut buffer = vec![0u16; INSTANCE_ID_BUFFER_CHARS];
    let mut required = 0;
    let success = unsafe {
        SetupDiGetDeviceInstanceIdW(
            device_set,
            info,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            &mut required,
        )
    };
    (success != 0).then(|| decode_utf16(&buffer[..buffer_len(required, buffer.len())]))
}

fn read_string_property(
    device_set: HDEVINFO,
    info: &mut SP_DEVINFO_DATA,
    property: u32,
) -> Option<String> {
    read_property(device_set, info, property)
        .and_then(|value| decode_multi_sz(&value).into_iter().next())
}

fn read_multi_string_property(
    device_set: HDEVINFO,
    info: &mut SP_DEVINFO_DATA,
    property: u32,
) -> Vec<String> {
    read_property(device_set, info, property)
        .map(|value| decode_multi_sz(&value))
        .unwrap_or_default()
}

fn read_property(
    device_set: HDEVINFO,
    info: &mut SP_DEVINFO_DATA,
    property: u32,
) -> Option<Vec<u16>> {
    let mut bytes = vec![0u8; PROPERTY_BUFFER_BYTES];
    let mut required = 0;
    let success = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            device_set,
            info,
            property,
            null_mut(),
            bytes.as_mut_ptr(),
            bytes.len() as u32,
            &mut required,
        )
    };
    if success == 0 || required == 0 {
        return None;
    }
    let byte_len = (required as usize).min(bytes.len());
    Some(
        bytes[..byte_len]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect(),
    )
}

fn decode_multi_sz(value: &[u16]) -> Vec<String> {
    value
        .split(|character| *character == 0)
        .filter(|part| !part.is_empty())
        .map(decode_utf16)
        .filter(|part| !part.is_empty())
        .collect()
}

fn decode_utf16(value: &[u16]) -> String {
    String::from_utf16_lossy(value)
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

fn buffer_len(required_with_null: u32, capacity: usize) -> usize {
    (required_with_null.saturating_sub(1) as usize).min(capacity)
}

fn last_error(context: &str) -> String {
    let error = unsafe { GetLastError() };
    format!("{context} (error {error})")
}

#[cfg(test)]
mod tests {
    use super::{
        buffer_len, decode_multi_sz, device_condition, enumerate_devices,
        is_explicit_generic_microsoft, signature_status,
    };
    use crate::domain::{DeviceCondition, SignatureStatus};
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        SIGNERSCORE_AUTHENTICODE, SIGNERSCORE_LOGO_STANDARD, SIGNERSCORE_UNSIGNED,
    };

    #[test]
    fn decodes_windows_multi_string_values() {
        let value: Vec<u16> = "PCI\\VEN_1002\0PCI\\VEN_1002&DEV_164E\0\0"
            .encode_utf16()
            .collect();
        assert_eq!(
            decode_multi_sz(&value),
            ["PCI\\VEN_1002", "PCI\\VEN_1002&DEV_164E"]
        );
    }

    #[test]
    fn excludes_instance_id_terminator_without_exceeding_capacity() {
        assert_eq!(buffer_len(8, 16), 7);
        assert_eq!(buffer_len(32, 16), 16);
        assert_eq!(buffer_len(0, 16), 0);
    }

    #[test]
    fn enumerates_present_devices_on_windows() {
        let devices = enumerate_devices().expect("Windows device enumeration should succeed");
        assert!(!devices.is_empty());
        assert!(devices.iter().all(|device| !device.instance_id.is_empty()));
        assert!(devices.iter().any(|device| {
            device.installed_driver.as_ref().is_some_and(|driver| {
                driver.provider.is_some()
                    && driver.version.is_some()
                    && driver.published_inf_name.is_some()
            })
        }));
    }

    #[test]
    fn maps_windows_signature_scores_without_guessing() {
        assert_eq!(
            signature_status(Some(SIGNERSCORE_LOGO_STANDARD)),
            SignatureStatus::Whql
        );
        assert_eq!(
            signature_status(Some(SIGNERSCORE_AUTHENTICODE)),
            SignatureStatus::Authenticode
        );
        assert_eq!(
            signature_status(Some(SIGNERSCORE_UNSIGNED)),
            SignatureStatus::Unsigned
        );
        assert_eq!(signature_status(Some(0x1234)), SignatureStatus::Unknown);
    }

    #[test]
    fn only_marks_explicit_microsoft_generic_drivers() {
        assert!(is_explicit_generic_microsoft(
            Some("Microsoft"),
            Some("Generic monitor"),
            "Monitor"
        ));
        assert!(!is_explicit_generic_microsoft(
            Some("Microsoft"),
            Some("High Definition Audio Device"),
            "Audio"
        ));
        assert!(!is_explicit_generic_microsoft(
            Some("Contoso"),
            Some("Generic adapter"),
            "Adapter"
        ));
    }

    #[test]
    fn distinguishes_missing_driver_from_other_device_problems() {
        assert_eq!(
            device_condition(Some(28), None, false, true),
            DeviceCondition::Missing
        );
        assert_eq!(
            device_condition(Some(10), None, true, true),
            DeviceCondition::Problem
        );
        assert_eq!(
            device_condition(None, None, true, true),
            DeviceCondition::Current
        );
    }
}
