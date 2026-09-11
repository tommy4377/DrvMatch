use std::{
    mem::size_of,
    ptr::{null, null_mut},
};

use windows_sys::Win32::{
    Devices::DeviceAndDriverInstallation::{
        DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SP_DEVINFO_DATA, SPDRP_CLASS, SPDRP_CLASSGUID,
        SPDRP_COMPATIBLEIDS, SPDRP_DEVICEDESC, SPDRP_FRIENDLYNAME, SPDRP_HARDWAREID, SPDRP_MFG,
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceRegistryPropertyW,
    },
    Foundation::{ERROR_NO_MORE_ITEMS, GetLastError, INVALID_HANDLE_VALUE},
};

use crate::domain::Device;

const PROPERTY_BUFFER_BYTES: usize = 32 * 1024;
const INSTANCE_ID_BUFFER_CHARS: usize = 4096;

struct DeviceInfoSet(HDEVINFO);

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        unsafe { SetupDiDestroyDeviceInfoList(self.0) };
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
        let friendly_name = read_string_property(device_set.0, &mut info, SPDRP_FRIENDLYNAME)
            .unwrap_or_else(|| description.clone());

        devices.push(Device {
            instance_id,
            friendly_name,
            description,
            manufacturer: read_string_property(device_set.0, &mut info, SPDRP_MFG),
            class_name: read_string_property(device_set.0, &mut info, SPDRP_CLASS),
            class_guid: read_string_property(device_set.0, &mut info, SPDRP_CLASSGUID),
            hardware_ids: read_multi_string_property(device_set.0, &mut info, SPDRP_HARDWAREID),
            compatible_ids: read_multi_string_property(
                device_set.0,
                &mut info,
                SPDRP_COMPATIBLEIDS,
            ),
            present: true,
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
    let char_len = byte_len / size_of::<u16>();
    let pointer = bytes.as_ptr().cast::<u16>();
    Some(unsafe { std::slice::from_raw_parts(pointer, char_len) }.to_vec())
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
    use super::{buffer_len, decode_multi_sz, enumerate_devices};

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
    }
}
