use std::{
    collections::{BTreeSet, HashMap},
    io::{Cursor, Read},
    time::Duration,
};

use cab::Cabinet;
use reqwest::{Url, blocking::Client};
use roxmltree::{Document, Node};

use crate::{
    domain::{
        CandidateCompatibility, CompatibilityState, Device, DriverCandidate, DriverSourceKind,
        MachineIdentity, SignatureStatus, SourceHealth, SourceHealthState,
    },
    metadata_cache::MetadataCache,
};

use super::http::{get_with_retry, read_limited, read_text_limited};

const USER_AGENT: &str = concat!("DrvMatch/", env!("CARGO_PKG_VERSION"));
const OEM_CACHE_SECONDS: i64 = 24 * 60 * 60;
const DELL_BASE_URL: &str = "https://dl.dell.com/";
const DELL_INDEX_URL: &str = "https://dl.dell.com/catalog/CatalogIndexPC.cab";
const HP_PLATFORM_LIST_URL: &str = "https://hpia.hpcloud.hp.com/ref/platformList.cab";
const LENOVO_DESCRIPTOR_LIMIT: usize = 256;
const MAX_OEM_BINARY_BYTES: u64 = 128 * 1024 * 1024;
const MAX_OEM_TEXT_BYTES: u64 = 32 * 1024 * 1024;

pub fn collect(
    kind: DriverSourceKind,
    machine: &MachineIdentity,
    device: &Device,
    cache: &MetadataCache,
    checked_at: i64,
) -> (Vec<DriverCandidate>, SourceHealth) {
    if !machine_matches_source(kind, machine) {
        return (
            Vec::new(),
            SourceHealth {
                source: kind,
                state: SourceHealthState::Skipped,
                checked_at,
                cached: false,
                candidate_count: 0,
                message: Some(if machine_manufacturer(machine).is_empty() {
                    "Machine manufacturer could not be identified, so this OEM source was not queried."
                        .into()
                } else {
                    "Not applicable to the detected machine manufacturer.".into()
                }),
            },
        );
    }

    let Some(cache_key) = machine_cache_key(kind, machine) else {
        return (
            Vec::new(),
            SourceHealth {
                source: kind,
                state: SourceHealthState::Skipped,
                checked_at,
                cached: false,
                candidate_count: 0,
                message: Some(
                    "The OEM was detected, but DrvMatch could not derive a stable model/platform identifier."
                        .into(),
                ),
            },
        );
    };

    let namespace = match kind {
        DriverSourceKind::Dell => "dell-oem-v2",
        DriverSourceKind::Lenovo => "lenovo-oem-v2",
        DriverSourceKind::Hp => "hp-oem-v2",
        _ => "unsupported-oem-v1",
    };

    match cache.get::<Vec<DriverCandidate>>(namespace, &cache_key) {
        Ok(Some(cached)) => {
            let candidates = filter_for_device(cached.value, device);
            let count = candidates.len();
            (
                candidates,
                SourceHealth {
                    source: kind,
                    state: SourceHealthState::Available,
                    checked_at: cached.fetched_at,
                    cached: true,
                    candidate_count: count,
                    message: Some(oem_health_message(kind, machine, count)),
                },
            )
        }
        Ok(None) => {
            let source = match OemSource::new(kind) {
                Ok(source) => source,
                Err(error) => {
                    return (
                        Vec::new(),
                        SourceHealth {
                            source: kind,
                            state: SourceHealthState::Failed,
                            checked_at,
                            cached: false,
                            candidate_count: 0,
                            message: Some(error),
                        },
                    );
                }
            };
            match source.discover_machine(machine, checked_at) {
                Ok(discovered) => {
                    let cache_error = cache
                        .put(namespace, &cache_key, OEM_CACHE_SECONDS, &discovered)
                        .err();
                    let candidates = filter_for_device(discovered, device);
                    let count = candidates.len();
                    let mut message = oem_health_message(kind, machine, count);
                    if let Some(error) = cache_error {
                        message.push_str(&format!(" Metadata cache warning: {error}"));
                    }
                    (
                        candidates,
                        SourceHealth {
                            source: kind,
                            state: SourceHealthState::Available,
                            checked_at,
                            cached: false,
                            candidate_count: count,
                            message: Some(message),
                        },
                    )
                }
                Err(error) => (
                    Vec::new(),
                    SourceHealth {
                        source: kind,
                        state: SourceHealthState::Failed,
                        checked_at,
                        cached: false,
                        candidate_count: 0,
                        message: Some(error),
                    },
                ),
            }
        }
        Err(error) => (
            Vec::new(),
            SourceHealth {
                source: kind,
                state: SourceHealthState::Failed,
                checked_at,
                cached: false,
                candidate_count: 0,
                message: Some(error),
            },
        ),
    }
}

#[derive(Clone)]
struct OemSource {
    kind: DriverSourceKind,
    client: Client,
}

impl OemSource {
    fn new(kind: DriverSourceKind) -> Result<Self, String> {
        if !matches!(
            kind,
            DriverSourceKind::Dell | DriverSourceKind::Lenovo | DriverSourceKind::Hp
        ) {
            return Err("The requested source is not an OEM catalog source.".into());
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(35))
            .redirect(reqwest::redirect::Policy::limited(6))
            .user_agent(USER_AGENT)
            .build()
            .map_err(|error| format!("Could not initialize the OEM catalog client: {error}"))?;
        Ok(Self { kind, client })
    }

    fn discover_machine(
        &self,
        machine: &MachineIdentity,
        retrieved_at: i64,
    ) -> Result<Vec<DriverCandidate>, String> {
        match self.kind {
            DriverSourceKind::Dell => self.discover_dell(machine, retrieved_at),
            DriverSourceKind::Lenovo => self.discover_lenovo(machine, retrieved_at),
            DriverSourceKind::Hp => self.discover_hp(machine, retrieved_at),
            _ => Err("The requested source is not an OEM catalog source.".into()),
        }
    }

    fn fetch_bytes(&self, url: &str, label: &str) -> Result<Vec<u8>, String> {
        let url = validate_oem_metadata_url(self.kind, url)?;
        let response = get_with_retry(|| self.client.get(&url), &format!("{label} request"))?;
        validate_oem_metadata_url(self.kind, response.url().as_str())?;
        read_limited(response, MAX_OEM_BINARY_BYTES, label)
    }

    fn fetch_text(&self, url: &str, label: &str) -> Result<String, String> {
        let url = validate_oem_metadata_url(self.kind, url)?;
        let response = get_with_retry(|| self.client.get(&url), &format!("{label} request"))?;
        validate_oem_metadata_url(self.kind, response.url().as_str())?;
        read_text_limited(response, MAX_OEM_TEXT_BYTES, label)
    }

    fn discover_dell(
        &self,
        machine: &MachineIdentity,
        retrieved_at: i64,
    ) -> Result<Vec<DriverCandidate>, String> {
        let index = self.fetch_bytes(DELL_INDEX_URL, "Dell CatalogIndexPC")?;
        let index_xml = extract_xml_from_cab(index, &["CatalogIndexPC.xml"])?;
        let manifest_path = dell_manifest_path(&index_xml, machine).ok_or_else(|| {
            format!(
                "Dell CatalogIndexPC does not contain an exact model/SKU entry for {}.",
                machine_label(machine)
            )
        })?;
        let manifest_url = force_https_dell_url(&absolute_url(DELL_BASE_URL, &manifest_path)?)?;
        let model_cab = self.fetch_bytes(&manifest_url, "Dell model catalog")?;
        let model_xml = extract_xml_from_cab(model_cab, &[])?;
        parse_dell_model_catalog(&model_xml, &manifest_url, machine, retrieved_at)
    }

    fn discover_lenovo(
        &self,
        machine: &MachineIdentity,
        retrieved_at: i64,
    ) -> Result<Vec<DriverCandidate>, String> {
        let machine_type = lenovo_machine_type(machine).ok_or_else(|| {
            format!(
                "Lenovo machine type could not be derived from {}.",
                machine_label(machine)
            )
        })?;
        let catalog_url = format!("https://download.lenovo.com/catalog/{machine_type}_Win11.xml");
        let catalog_xml = self.fetch_text(&catalog_url, "Lenovo machine catalog")?;
        let descriptor_urls = parse_lenovo_catalog_locations(&catalog_xml)?;
        if descriptor_urls.is_empty() {
            return Ok(Vec::new());
        }

        let descriptor_urls = descriptor_urls
            .into_iter()
            .take(LENOVO_DESCRIPTOR_LIMIT)
            .collect::<Vec<_>>();
        let worker_count = descriptor_urls.len().clamp(1, 8);
        let chunk_size = descriptor_urls.len().div_ceil(worker_count);
        let outcomes = std::thread::scope(|scope| {
            let handles = descriptor_urls
                .chunks(chunk_size)
                .map(|chunk| {
                    let source = self.clone();
                    let machine_type = machine_type.clone();
                    scope.spawn(move || {
                        let mut candidates = Vec::new();
                        let mut successes = 0usize;
                        let mut last_error = None;
                        for descriptor_url in chunk {
                            match source.fetch_text(descriptor_url, "Lenovo package descriptor") {
                                Ok(xml) => {
                                    successes += 1;
                                    match parse_lenovo_descriptor(
                                        &xml,
                                        descriptor_url,
                                        machine,
                                        &machine_type,
                                        retrieved_at,
                                    ) {
                                        Ok(Some(candidate)) => candidates.push(candidate),
                                        Ok(None) => {}
                                        Err(error) => last_error = Some(error),
                                    }
                                }
                                Err(error) => last_error = Some(error),
                            }
                        }
                        (candidates, successes, last_error)
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| {
                    handle.join().unwrap_or_else(|_| {
                        (
                            Vec::new(),
                            0,
                            Some("A Lenovo catalog worker terminated unexpectedly.".into()),
                        )
                    })
                })
                .collect::<Vec<_>>()
        });

        let mut candidates = Vec::new();
        let mut descriptor_successes = 0usize;
        let mut last_error = None;
        for (mut discovered, successes, error) in outcomes {
            candidates.append(&mut discovered);
            descriptor_successes += successes;
            if error.is_some() {
                last_error = error;
            }
        }
        if descriptor_successes == 0 {
            return Err(last_error.unwrap_or_else(|| {
                "Lenovo returned package descriptors, but none could be downloaded.".into()
            }));
        }
        Ok(candidates)
    }

    fn discover_hp(
        &self,
        machine: &MachineIdentity,
        retrieved_at: i64,
    ) -> Result<Vec<DriverCandidate>, String> {
        let platform = hp_platform_id(machine).ok_or_else(|| {
            format!(
                "HP platform ID could not be derived from the baseboard product for {}.",
                machine_label(machine)
            )
        })?;
        let platform_cab = self.fetch_bytes(HP_PLATFORM_LIST_URL, "HP HPIA platform list")?;
        let platform_xml = extract_xml_from_cab(platform_cab, &["platformList.xml"])?;
        let releases = hp_windows_11_releases(&platform_xml, &platform);
        if releases.is_empty() {
            return Err(format!(
                "HP HPIA does not list a Windows 11 reference image for platform {platform}."
            ));
        }
        let release = select_hp_windows_11_release(&releases, machine, &platform)?;
        let reference_url =
            format!("https://hpia.hpcloud.hp.com/ref/{platform}/{platform}_64_11.0.{release}.cab");
        let reference_cab = self.fetch_bytes(&reference_url, "HP HPIA reference catalog")?;
        let reference_xml = extract_xml_from_cab(reference_cab, &[])?;
        parse_hp_reference(
            &reference_xml,
            &reference_url,
            machine,
            &platform,
            retrieved_at,
        )
    }
}

fn filter_for_device(candidates: Vec<DriverCandidate>, device: &Device) -> Vec<DriverCandidate> {
    let device_ids = device
        .hardware_ids
        .iter()
        .chain(device.compatible_ids.iter())
        .map(|value| normalize_id(value))
        .collect::<BTreeSet<_>>();
    candidates
        .into_iter()
        .filter(|candidate| {
            candidate
                .hardware_ids
                .iter()
                .chain(candidate.compatible_ids.iter())
                .any(|value| device_ids.contains(&normalize_id(value)))
        })
        .collect()
}

fn machine_matches_source(kind: DriverSourceKind, machine: &MachineIdentity) -> bool {
    let manufacturer = machine_manufacturer(machine);
    match kind {
        DriverSourceKind::Dell => {
            manufacturer.contains("dell") || manufacturer.contains("alienware")
        }
        DriverSourceKind::Lenovo => manufacturer.contains("lenovo"),
        DriverSourceKind::Hp => {
            manufacturer.contains("hp")
                || manufacturer.contains("hewlett")
                || manufacturer.contains("compaq")
        }
        _ => false,
    }
}

fn machine_manufacturer(machine: &MachineIdentity) -> String {
    machine
        .manufacturer
        .as_deref()
        .or(machine.baseboard_manufacturer.as_deref())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn machine_cache_key(kind: DriverSourceKind, machine: &MachineIdentity) -> Option<String> {
    let value = match kind {
        DriverSourceKind::Dell => machine
            .system_sku
            .clone()
            .or_else(|| machine.model.clone())
            .or_else(|| machine.baseboard_product.clone()),
        DriverSourceKind::Lenovo => lenovo_machine_type(machine),
        DriverSourceKind::Hp => hp_platform_id(machine),
        _ => None,
    }?;
    let os_release = machine
        .windows_display_version
        .as_deref()
        .map(normalize_machine_value)
        .unwrap_or_else(|| "unknown-windows-release".into());
    Some(format!(
        "{:?}:{}:{}",
        kind,
        normalize_machine_value(&value),
        os_release
    ))
}

fn machine_label(machine: &MachineIdentity) -> String {
    machine
        .model
        .as_deref()
        .or(machine.system_sku.as_deref())
        .or(machine.baseboard_product.as_deref())
        .unwrap_or("the detected machine")
        .to_string()
}

fn oem_health_message(kind: DriverSourceKind, machine: &MachineIdentity, count: usize) -> String {
    let source = match kind {
        DriverSourceKind::Dell => "Dell",
        DriverSourceKind::Lenovo => "Lenovo",
        DriverSourceKind::Hp => "HP",
        _ => "OEM",
    };
    if count == 0 {
        format!(
            "{source} catalog matched {}, but returned no package with an ID matching this device.",
            machine_label(machine)
        )
    } else {
        format!(
            "{source} catalog matched {} and returned {count} device-specific candidate{}.",
            machine_label(machine),
            if count == 1 { "" } else { "s" }
        )
    }
}

fn extract_xml_from_cab(bytes: Vec<u8>, preferred_names: &[&str]) -> Result<String, String> {
    let cursor = Cursor::new(bytes);
    let mut cabinet = Cabinet::new(cursor)
        .map_err(|error| format!("Could not open the OEM CAB metadata: {error}"))?;
    let names = cabinet
        .folder_entries()
        .flat_map(|folder| folder.file_entries().map(|entry| entry.name().to_string()))
        .collect::<Vec<_>>();
    let selected = preferred_names
        .iter()
        .find_map(|preferred| {
            names
                .iter()
                .find(|name| name.eq_ignore_ascii_case(preferred))
                .cloned()
        })
        .or_else(|| {
            names
                .iter()
                .find(|name| name.to_ascii_lowercase().ends_with(".xml"))
                .cloned()
        })
        .ok_or_else(|| "The OEM CAB metadata contains no XML document.".to_string())?;
    let mut reader = cabinet
        .read_file(&selected)
        .map_err(|error| format!("Could not open {selected} inside the OEM CAB: {error}"))?;
    let mut payload = Vec::new();
    reader
        .read_to_end(&mut payload)
        .map_err(|error| format!("Could not extract {selected} from the OEM CAB: {error}"))?;
    decode_xml_bytes(&payload)
}

fn decode_xml_bytes(bytes: &[u8]) -> Result<String, String> {
    if bytes.starts_with(&[0xff, 0xfe]) {
        if !(bytes.len() - 2).is_multiple_of(2) {
            return Err("The OEM XML contains malformed UTF-16LE data.".into());
        }
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units)
            .map_err(|error| format!("The OEM XML is not valid UTF-16LE: {error}"));
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        if !(bytes.len() - 2).is_multiple_of(2) {
            return Err("The OEM XML contains malformed UTF-16BE data.".into());
        }
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units)
            .map_err(|error| format!("The OEM XML is not valid UTF-16BE: {error}"));
    }
    String::from_utf8(bytes.to_vec())
        .map(|value| value.trim_start_matches('\u{feff}').to_string())
        .map_err(|error| format!("The OEM XML is not valid UTF-8: {error}"))
}

fn dell_manifest_path(xml: &str, machine: &MachineIdentity) -> Option<String> {
    let document = Document::parse(xml.trim_start_matches('\u{feff}')).ok()?;
    document
        .descendants()
        .filter(|node| tag_is(*node, "GroupManifest"))
        .find(|group| {
            group
                .descendants()
                .filter(|node| tag_is(*node, "Model"))
                .any(|model| dell_model_matches(model, machine))
        })
        .and_then(|group| {
            group
                .descendants()
                .find(|node| tag_is(*node, "ManifestInformation"))
        })
        .and_then(|info| attr_ci(info, "path").or_else(|| descendant_text(info, "path")))
}

fn dell_model_matches(model: Node<'_, '_>, machine: &MachineIdentity) -> bool {
    let system_id = attr_ci(model, "systemID");
    let name = attr_ci(model, "name");
    let id_matches = [
        machine.system_sku.as_deref(),
        machine.baseboard_product.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| {
        system_id
            .as_deref()
            .is_some_and(|id| machine_values_equal(id, value))
    });
    let name_matches = [machine.model.as_deref(), machine.system_family.as_deref()]
        .into_iter()
        .flatten()
        .any(|value| {
            name.as_deref()
                .is_some_and(|candidate| machine_values_equal(candidate, value))
        });
    id_matches || name_matches
}

fn parse_dell_model_catalog(
    xml: &str,
    catalog_url: &str,
    machine: &MachineIdentity,
    retrieved_at: i64,
) -> Result<Vec<DriverCandidate>, String> {
    let document = Document::parse(xml.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("Dell model catalog XML is invalid: {error}"))?;
    let mut results = Vec::new();
    for (index, component) in document
        .descendants()
        .filter(|node| tag_is(*node, "SoftwareComponent"))
        .enumerate()
    {
        if component_has_other_model(component, machine) {
            continue;
        }
        let component_type = component
            .descendants()
            .find(|node| tag_is(*node, "ComponentType"))
            .and_then(|node| attr_ci(node, "value"))
            .unwrap_or_default()
            .to_ascii_uppercase();
        if ["BIOS", "FRMW", "APAC"]
            .iter()
            .any(|value| component_type.contains(value))
        {
            continue;
        }

        let mut hardware_ids = dell_pci_hardware_ids(component);
        hardware_ids.extend(extract_hardware_ids_from_node(component));
        hardware_ids = unique_strings(hardware_ids);
        if hardware_ids.is_empty() {
            continue;
        }

        let Some(path) = attr_ci(component, "path").or_else(|| descendant_text(component, "path"))
        else {
            continue;
        };
        let download_url = force_https_dell_url(&absolute_url(DELL_BASE_URL, &path)?)?;
        let source_id = attr_ci(component, "releaseID")
            .or_else(|| attr_ci(component, "packageID"))
            .unwrap_or_else(|| format!("component-{index}"));
        let display_name = descendant_display(component, "Name")
            .or_else(|| descendant_display(component, "Category"))
            .unwrap_or_else(|| format!("Dell driver {source_id}"));
        let version = attr_ci(component, "vendorVersion")
            .or_else(|| attr_ci(component, "dellVersion"))
            .or_else(|| descendant_text(component, "VendorVersion"));
        let publication_date = attr_ci(component, "dateTime")
            .or_else(|| attr_ci(component, "releaseDate"))
            .or_else(|| descendant_text(component, "ReleaseDate"));
        let release_channel = descendant_display(component, "Criticality")
            .or_else(|| descendant_text(component, "Criticality"));
        let provider = descendant_display(component, "Vendor")
            .or_else(|| descendant_text(component, "Vendor"))
            .or_else(|| Some("Dell".into()));
        let class_name = descendant_display(component, "Category");
        let expected_sha256 = find_sha256(component);
        let size_bytes = attr_ci(component, "size")
            .and_then(|value| value.parse::<u64>().ok())
            .or_else(|| descendant_text(component, "Size").and_then(|value| value.parse().ok()));
        let (supported_os, supported_architectures) = dell_supported_os(component);
        if !dell_windows_11_applies(&supported_os, machine) {
            continue;
        }
        let release_notes_url = component
            .descendants()
            .find(|node| tag_is(*node, "ReleaseNotes") || tag_is(*node, "ReleaseNotesURL"))
            .and_then(node_path_or_text)
            .and_then(|value| absolute_url(DELL_BASE_URL, &value).ok())
            .and_then(|value| force_https_dell_url(&value).ok());

        results.push(DriverCandidate {
            id: format!(
                "dell:{source_id}:{}",
                version.as_deref().unwrap_or("package")
            ),
            source: DriverSourceKind::Dell,
            source_specific_id: source_id,
            alternate_sources: Vec::new(),
            display_name,
            provider,
            manufacturer: Some("Dell".into()),
            version,
            version_is_package_version: false,
            driver_date: None,
            publication_date,
            class_name,
            supported_os,
            supported_architectures,
            hardware_ids,
            compatible_ids: Vec::new(),
            download_url: Some(download_url),
            details_url: Some(catalog_url.into()),
            release_notes_url,
            release_channel,
            oem_models: machine_oem_labels(machine),
            known_issues: Vec::new(),
            known_regressions: Vec::new(),
            fixed_issues: Vec::new(),
            security_relevant: false,
            signature: SignatureStatus::Unknown,
            expected_sha256,
            package_type: package_type_from_url(&path),
            package_group: Some("Dell OEM driver".into()),
            size_bytes,
            retrieved_at,
            compatibility: unknown_compatibility(),
        });
    }
    Ok(results)
}

fn component_has_other_model(component: Node<'_, '_>, machine: &MachineIdentity) -> bool {
    let models = component
        .descendants()
        .filter(|node| tag_is(*node, "Model"))
        .collect::<Vec<_>>();
    !models.is_empty()
        && !models
            .into_iter()
            .any(|model| dell_model_matches(model, machine))
}

fn dell_pci_hardware_ids(component: Node<'_, '_>) -> Vec<String> {
    let mut ids = Vec::new();
    for pci in component
        .descendants()
        .filter(|node| tag_is(*node, "PCIInfo"))
    {
        let Some(vendor) = attr_ci(pci, "vendorID").map(clean_hex) else {
            continue;
        };
        let Some(device) = attr_ci(pci, "deviceID").map(clean_hex) else {
            continue;
        };
        let generic = format!("PCI\\VEN_{vendor}&DEV_{device}");
        let sub_vendor = attr_ci(pci, "subVendorID").map(clean_hex);
        let sub_device = attr_ci(pci, "subDeviceID").map(clean_hex);
        if let (Some(sub_device), Some(sub_vendor)) = (sub_device, sub_vendor) {
            ids.push(format!("{generic}&SUBSYS_{sub_device}{sub_vendor}"));
        }
        ids.push(generic);
    }
    ids
}

fn dell_supported_os(component: Node<'_, '_>) -> (Vec<String>, Vec<String>) {
    let mut systems = Vec::new();
    let mut architectures = Vec::new();
    for os in component
        .descendants()
        .filter(|node| tag_is(*node, "OperatingSystem"))
    {
        let mut parts = Vec::new();
        if let Some(display) = descendant_text(os, "Display") {
            parts.push(display);
        }
        for name in [
            "osCode",
            "osVendor",
            "majorVersion",
            "minorVersion",
            "spMajorVersion",
        ] {
            if let Some(value) = attr_ci(os, name) {
                parts.push(value);
            }
        }
        if !parts.is_empty() {
            systems.push(parts.join(" "));
        }
        if let Some(arch) = attr_ci(os, "osArch").or_else(|| attr_ci(os, "architecture")) {
            architectures.push(normalize_architecture(&arch));
        }
    }
    if architectures.is_empty() {
        architectures.push("x64".into());
    }
    (unique_strings(systems), unique_strings(architectures))
}

fn parse_lenovo_catalog_locations(xml: &str) -> Result<Vec<String>, String> {
    let document = Document::parse(xml.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("Lenovo machine catalog XML is invalid: {error}"))?;
    Ok(unique_strings(
        document
            .descendants()
            .filter(|node| tag_is(*node, "location"))
            .filter_map(|node| node.text().map(str::trim))
            .filter_map(|value| force_https_lenovo_url(value).ok())
            .collect(),
    ))
}

fn parse_lenovo_descriptor(
    xml: &str,
    descriptor_url: &str,
    machine: &MachineIdentity,
    machine_type: &str,
    retrieved_at: i64,
) -> Result<Option<DriverCandidate>, String> {
    let document = Document::parse(xml.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("Lenovo package descriptor XML is invalid: {error}"))?;
    let Some(package) = document.descendants().find(|node| tag_is(*node, "Package")) else {
        return Ok(None);
    };
    let package_type = package
        .descendants()
        .find(|node| tag_is(*node, "PackageType"))
        .and_then(|node| attr_ci(node, "type").or_else(|| node.text().map(str::to_string)));
    if !package_type
        .as_deref()
        .is_some_and(|value| value.trim() == "2" || value.trim().eq_ignore_ascii_case("driver"))
    {
        return Ok(None);
    }

    let mut hardware_ids = extract_hardware_ids_from_node(package);
    hardware_ids = unique_strings(hardware_ids);
    if hardware_ids.is_empty() {
        return Ok(None);
    }

    let package_id = attr_ci(package, "id").unwrap_or_else(|| {
        descriptor_url
            .rsplit('/')
            .next()
            .unwrap_or("package")
            .trim_end_matches(".xml")
            .to_string()
    });
    let name = attr_ci(package, "name")
        .or_else(|| descendant_text(package, "Name"))
        .or_else(|| descendant_text(package, "Desc"))
        .unwrap_or_else(|| format!("Lenovo driver {package_id}"));
    let lower_name = name.to_ascii_lowercase();
    if lower_name.contains("bios") || lower_name.contains("firmware") {
        return Ok(None);
    }

    let installer = package
        .descendants()
        .find(|node| tag_is(*node, "Installer"));
    let installer_file =
        installer.and_then(|node| node.descendants().find(|child| tag_is(*child, "File")));
    let installer_name = installer_file
        .and_then(|node| attr_ci(node, "name").or_else(|| descendant_text(node, "Name")))
        .or_else(|| descendant_text(package, "FileName"));
    let Some(installer_name) = installer_name else {
        return Ok(None);
    };
    let download_url =
        if installer_name.starts_with("https://") || installer_name.starts_with("http://") {
            force_https_lenovo_url(&installer_name)?
        } else {
            let joined = Url::parse(descriptor_url)
                .and_then(|base| base.join(&installer_name))
                .map(|value| value.to_string())
                .map_err(|error| format!("Lenovo installer URL is invalid: {error}"))?;
            force_https_lenovo_url(&joined)?
        };
    let release_notes_url = package
        .descendants()
        .find(|node| tag_is(*node, "Readme"))
        .and_then(|node| node.descendants().find(|child| tag_is(*child, "File")))
        .and_then(|node| attr_ci(node, "name").or_else(|| descendant_text(node, "Name")))
        .and_then(|name| Url::parse(descriptor_url).ok()?.join(&name).ok())
        .and_then(|url| force_https_lenovo_url(url.as_str()).ok());
    let version = attr_ci(package, "version").or_else(|| descendant_text(package, "Version"));
    let publication_date = descendant_text(package, "ReleaseDate");
    let provider = descendant_text(package, "Vendor").or_else(|| Some("Lenovo".into()));
    let severity = package
        .descendants()
        .find(|node| tag_is(*node, "Severity"))
        .and_then(|node| attr_ci(node, "type").or_else(|| node.text().map(str::to_string)))
        .map(|value| lenovo_severity(&value));
    let size_bytes = installer_file
        .and_then(|node| attr_ci(node, "size").or_else(|| descendant_text(node, "Size")))
        .and_then(|value| value.parse::<u64>().ok());
    let expected_sha256 = installer_file
        .and_then(find_sha256)
        .or_else(|| find_sha256(package));

    Ok(Some(DriverCandidate {
        id: format!(
            "lenovo:{package_id}:{}",
            version.as_deref().unwrap_or("package")
        ),
        source: DriverSourceKind::Lenovo,
        source_specific_id: package_id,
        alternate_sources: Vec::new(),
        display_name: name,
        provider,
        manufacturer: Some("Lenovo".into()),
        version,
        version_is_package_version: false,
        driver_date: None,
        publication_date,
        class_name: None,
        supported_os: vec!["Windows 11 x64".into()],
        supported_architectures: vec!["x64".into()],
        hardware_ids,
        compatible_ids: Vec::new(),
        download_url: Some(download_url.clone()),
        details_url: Some(descriptor_url.into()),
        release_notes_url,
        release_channel: severity,
        oem_models: unique_strings(
            machine_oem_labels(machine)
                .into_iter()
                .chain(std::iter::once(machine_type.into()))
                .collect(),
        ),
        known_issues: Vec::new(),
        known_regressions: Vec::new(),
        fixed_issues: Vec::new(),
        security_relevant: false,
        signature: SignatureStatus::Unknown,
        expected_sha256,
        package_type: package_type_from_url(&download_url),
        package_group: Some("Lenovo OEM driver".into()),
        size_bytes,
        retrieved_at,
        compatibility: unknown_compatibility(),
    }))
}

fn lenovo_severity(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "critical" => "Critical",
        "2" | "recommended" => "Recommended",
        "3" | "optional" => "Optional",
        _ => "OEM stable",
    }
    .into()
}

fn lenovo_machine_type(machine: &MachineIdentity) -> Option<String> {
    for value in [
        machine.system_sku.as_deref(),
        machine.model.as_deref(),
        machine.system_family.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        let upper = value.to_ascii_uppercase();
        if let Some(index) = upper.find("MT_") {
            let tail = &upper[index + 3..];
            let candidate = tail
                .chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .take(4)
                .collect::<String>();
            if candidate.len() == 4
                && candidate
                    .chars()
                    .any(|character| character.is_ascii_digit())
            {
                return Some(candidate);
            }
        }
        for token in upper.split(|character: char| !character.is_ascii_alphanumeric()) {
            if token.len() >= 4 && token.chars().any(|character| character.is_ascii_digit()) {
                return Some(token.chars().take(4).collect());
            }
        }
        let compact = upper
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .collect::<String>();
        if compact.len() >= 4
            && !compact.starts_with("LENOVO")
            && compact
                .chars()
                .take(4)
                .any(|character| character.is_ascii_digit())
        {
            return Some(compact.chars().take(4).collect());
        }
    }
    None
}

fn hp_platform_id(machine: &MachineIdentity) -> Option<String> {
    for value in [
        machine.baseboard_product.as_deref(),
        machine.system_sku.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        for token in value
            .to_ascii_uppercase()
            .split(|character: char| !character.is_ascii_alphanumeric())
        {
            if token.len() == 4 && token.chars().all(|character| character.is_ascii_hexdigit()) {
                return Some(token.into());
            }
        }
        let compact = value
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_uppercase();
        if compact.len() == 4
            && compact
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Some(compact);
        }
    }
    None
}

fn hp_windows_11_releases(xml: &str, platform: &str) -> Vec<String> {
    let Ok(document) = Document::parse(xml.trim_start_matches('\u{feff}')) else {
        return Vec::new();
    };
    let Some(platform_node) = document
        .descendants()
        .filter(|node| tag_is(*node, "Platform"))
        .find(|node| {
            descendant_text(*node, "SystemID")
                .is_some_and(|value| value.eq_ignore_ascii_case(platform))
        })
    else {
        return Vec::new();
    };
    let mut releases = platform_node
        .descendants()
        .filter(|node| tag_is(*node, "OS"))
        .filter_map(|node| {
            let description = descendant_text(node, "OSDescription").unwrap_or_default();
            let release = descendant_text(node, "OSReleaseIdDisplay")?;
            let combined = format!("{} {}", description, node_text(node)).to_ascii_lowercase();
            if combined.contains("windows 11")
                && !combined.contains("ltsb")
                && !combined.contains("ltsc")
            {
                Some(release)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    releases.sort_by_key(|release| std::cmp::Reverse(release_rank(release)));
    unique_strings(releases)
}

#[cfg(test)]
fn hp_latest_windows_11_release(xml: &str, platform: &str) -> Option<String> {
    hp_windows_11_releases(xml, platform).into_iter().next()
}

fn select_hp_windows_11_release(
    releases: &[String],
    machine: &MachineIdentity,
    platform: &str,
) -> Result<String, String> {
    let windows_release = machine
        .windows_display_version
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Windows DisplayVersion could not be read, so DrvMatch will not guess an HP HPIA reference release.".to_string()
        })?;
    releases
        .iter()
        .find(|release| release.eq_ignore_ascii_case(windows_release))
        .cloned()
        .ok_or_else(|| {
            format!(
                "HP HPIA does not list Windows {windows_release} for platform {platform}; DrvMatch will not use a different Windows release as a substitute."
            )
        })
}

fn release_rank(value: &str) -> u32 {
    let upper = value.to_ascii_uppercase();
    let digits = upper
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect::<String>();
    let year = digits
        .get(0..2)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let half = if upper.contains("H2") {
        2
    } else if upper.contains("H1") {
        1
    } else {
        0
    };
    year * 10 + half
}

fn parse_hp_reference(
    xml: &str,
    reference_url: &str,
    machine: &MachineIdentity,
    platform: &str,
    retrieved_at: i64,
) -> Result<Vec<DriverCandidate>, String> {
    let document = Document::parse(xml.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("HP HPIA reference XML is invalid: {error}"))?;
    let mut ids_by_solution: HashMap<String, Vec<String>> = HashMap::new();
    for device in document
        .descendants()
        .filter(|node| tag_is(*node, "Device"))
    {
        let ids = extract_hardware_ids_from_node(device);
        if ids.is_empty() {
            continue;
        }
        for reference in device
            .descendants()
            .filter(|node| tag_is(*node, "UpdateInfo"))
            .filter_map(|node| attr_ci(node, "IdRef"))
        {
            ids_by_solution
                .entry(reference)
                .or_default()
                .extend(ids.iter().cloned());
        }
    }

    let mut results = Vec::new();
    for update in document
        .descendants()
        .filter(|node| tag_is(*node, "UpdateInfo"))
        .filter(|node| attr_ci(*node, "IdRef").is_none())
    {
        let category = descendant_text(update, "Category").unwrap_or_default();
        let category_lower = category.to_ascii_lowercase();
        if !category_lower.contains("driver") || category_lower.contains("driver pack") {
            continue;
        }
        let Some(source_id) = attr_ci(update, "id").or_else(|| descendant_text(update, "Id"))
        else {
            continue;
        };
        let hardware_ids = unique_strings(ids_by_solution.remove(&source_id).unwrap_or_default());
        if hardware_ids.is_empty() {
            continue;
        }
        let Some(raw_url) = descendant_text(update, "Url") else {
            continue;
        };
        let download_url = force_https_hp_url(&raw_url)?;
        let display_name =
            descendant_text(update, "Name").unwrap_or_else(|| format!("HP driver {source_id}"));
        let version = descendant_text(update, "Version");
        let provider = descendant_text(update, "Vendor").or_else(|| Some("HP".into()));
        let release_channel =
            descendant_text(update, "ReleaseType").or_else(|| Some("OEM recommended".into()));
        let expected_sha256 =
            descendant_text(update, "SHA256").filter(|value| looks_like_sha256(value));
        let size_bytes =
            descendant_text(update, "Size").and_then(|value| value.parse::<u64>().ok());
        let release_notes_url = descendant_text(update, "ReleaseNotesUrl")
            .and_then(|value| force_https_hp_url(&value).ok());
        let supported_os = descendant_text(update, "SupportedOS")
            .map(|value| vec![value])
            .unwrap_or_else(|| vec!["Windows 11 x64".into()]);

        results.push(DriverCandidate {
            id: format!("hp:{source_id}:{}", version.as_deref().unwrap_or("package")),
            source: DriverSourceKind::Hp,
            source_specific_id: source_id,
            alternate_sources: Vec::new(),
            display_name,
            provider,
            manufacturer: Some("HP".into()),
            version,
            version_is_package_version: false,
            driver_date: None,
            publication_date: descendant_text(update, "DateReleased"),
            class_name: Some("Driver".into()),
            supported_os,
            supported_architectures: vec!["x64".into()],
            hardware_ids,
            compatible_ids: Vec::new(),
            download_url: Some(download_url.clone()),
            details_url: Some(reference_url.into()),
            release_notes_url,
            release_channel,
            oem_models: unique_strings(
                machine_oem_labels(machine)
                    .into_iter()
                    .chain(std::iter::once(platform.into()))
                    .collect(),
            ),
            known_issues: Vec::new(),
            known_regressions: Vec::new(),
            fixed_issues: Vec::new(),
            security_relevant: false,
            signature: SignatureStatus::Unknown,
            expected_sha256,
            package_type: package_type_from_url(&download_url),
            package_group: Some("HP SoftPaq driver".into()),
            size_bytes,
            retrieved_at,
            compatibility: unknown_compatibility(),
        });
    }
    Ok(results)
}

fn force_https_dell_url(value: &str) -> Result<String, String> {
    let value = value.trim().replacen("http://", "https://", 1);
    let url =
        Url::parse(&value).map_err(|error| format!("Dell catalog URL is invalid: {error}"))?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if url.scheme() != "https" || !(host == "dell.com" || host.ends_with(".dell.com")) {
        return Err("Dell catalog returned a URL outside the official Dell HTTPS domains.".into());
    }
    Ok(url.to_string())
}

fn force_https_lenovo_url(value: &str) -> Result<String, String> {
    let value = value.trim().replacen("http://", "https://", 1);
    let url =
        Url::parse(&value).map_err(|error| format!("Lenovo package URL is invalid: {error}"))?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if url.scheme() != "https" || !(host == "lenovo.com" || host.ends_with(".lenovo.com")) {
        return Err(
            "Lenovo catalog returned a URL outside the official Lenovo HTTPS domains.".into(),
        );
    }
    Ok(url.to_string())
}

fn force_https_hp_url(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let normalized = if trimmed.starts_with("http://") {
        trimmed.replacen("http://", "https://", 1)
    } else if trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.to_ascii_lowercase().starts_with("ftp.hp.com/")
        || trimmed.to_ascii_lowercase().starts_with("ftp.ext.hp.com/")
        || trimmed
            .to_ascii_lowercase()
            .starts_with("hpia.hpcloud.hp.com/")
    {
        format!("https://{trimmed}")
    } else {
        return Err("HP catalog returned a package URL without an official HP host.".into());
    };
    require_official_https_url(&normalized, &["hp.com"])
        .map_err(|error| format!("HP package URL is invalid: {error}"))
}

fn extract_hardware_ids_from_node(node: Node<'_, '_>) -> Vec<String> {
    let mut ids = Vec::new();
    for descendant in node.descendants() {
        for attribute in descendant.attributes() {
            ids.extend(extract_hardware_ids(attribute.value()));
        }
        if let Some(text) = descendant.text() {
            ids.extend(extract_hardware_ids(text));
        }
    }
    unique_strings(ids)
}

fn extract_hardware_ids(value: &str) -> Vec<String> {
    const PREFIXES: [&str; 10] = [
        "PCI\\",
        "USB\\",
        "ACPI\\",
        "HDAUDIO\\",
        "HID\\",
        "BTH\\",
        "SWD\\",
        "ROOT\\",
        "USBPRINT\\",
        "DISPLAY\\",
    ];
    let upper = value.to_ascii_uppercase();
    let mut ids = Vec::new();
    for prefix in PREFIXES {
        let mut offset = 0usize;
        while let Some(relative) = upper[offset..].find(prefix) {
            let start = offset + relative;
            let tail = &upper[start..];
            let end = tail
                .char_indices()
                .skip(1)
                .find_map(|(index, character)| {
                    (character.is_whitespace()
                        || matches!(character, '<' | '>' | '"' | '\'' | ',' | ';' | ')' | '('))
                    .then_some(index)
                })
                .unwrap_or(tail.len());
            let id = tail[..end]
                .trim_matches(|character: char| matches!(character, '.' | ':' | ']'))
                .to_string();
            if id.len() > prefix.len() {
                ids.push(id);
            }
            offset = start.saturating_add(end.max(1));
            if offset >= upper.len() {
                break;
            }
        }
    }
    unique_strings(ids)
}

fn find_sha256(node: Node<'_, '_>) -> Option<String> {
    for descendant in node.descendants() {
        for attribute in descendant.attributes() {
            if attribute.name().to_ascii_lowercase().contains("sha256")
                && looks_like_sha256(attribute.value())
            {
                return Some(attribute.value().trim().to_ascii_lowercase());
            }
        }
        let tag = descendant.tag_name().name().to_ascii_lowercase();
        let algorithm = attr_ci(descendant, "algorithm")
            .or_else(|| attr_ci(descendant, "type"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        if tag.contains("sha256")
            || ((tag == "hash" || tag == "checksum") && algorithm.contains("sha256"))
        {
            let text = node_text(descendant);
            if let Some(value) = text.split_whitespace().next()
                && looks_like_sha256(value)
            {
                return Some(value.to_ascii_lowercase());
            }
        }
    }
    None
}

fn looks_like_sha256(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.len() == 64
        && trimmed
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

fn package_type_from_url(value: &str) -> Option<String> {
    let path = Url::parse(value)
        .ok()
        .map(|url| url.path().to_string())
        .unwrap_or_else(|| value.to_string());
    path.rsplit('.')
        .next()
        .filter(|extension| {
            extension.len() <= 5 && extension.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .map(|extension| extension.to_ascii_uppercase())
}

fn machine_oem_labels(machine: &MachineIdentity) -> Vec<String> {
    unique_strings(
        [
            machine.model.clone(),
            machine.system_sku.clone(),
            machine.system_family.clone(),
            machine.baseboard_product.clone(),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

fn validate_oem_metadata_url(kind: DriverSourceKind, value: &str) -> Result<String, String> {
    let roots: &[&str] = match kind {
        DriverSourceKind::Dell => &["dell.com"],
        DriverSourceKind::Lenovo => &["lenovo.com"],
        DriverSourceKind::Hp => &["hp.com"],
        _ => return Err("The requested source is not an OEM metadata source.".into()),
    };
    require_official_https_url(value, roots)
        .map_err(|error| format!("OEM metadata URL was rejected: {error}"))
}

fn absolute_url(base: &str, value: &str) -> Result<String, String> {
    let joined = if value.starts_with("https://") {
        value.to_string()
    } else if value.starts_with("http://") {
        value.replacen("http://", "https://", 1)
    } else {
        Url::parse(base)
            .and_then(|base| base.join(value.trim_start_matches('/')))
            .map(|url| url.to_string())
            .map_err(|error| format!("OEM catalog URL is invalid: {error}"))?
    };
    let base_host = Url::parse(base)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .ok_or_else(|| "OEM catalog base URL has no host.".to_string())?;
    let root = registrable_vendor_root(&base_host)
        .ok_or_else(|| "OEM catalog base host is not recognized.".to_string())?;
    require_official_https_url(&joined, &[root])
}

fn require_official_https_url(value: &str, roots: &[&str]) -> Result<String, String> {
    let url = Url::parse(value).map_err(|error| format!("invalid URL: {error}"))?;
    if url.scheme() != "https" {
        return Err("only HTTPS URLs are accepted".into());
    }
    let host = url
        .host_str()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let allowed = roots.iter().any(|root| {
        let root = root.to_ascii_lowercase();
        host == root || host.ends_with(&format!(".{root}"))
    });
    if !allowed {
        return Err(format!(
            "host {host} is outside the expected official OEM domain"
        ));
    }
    Ok(url.to_string())
}

fn registrable_vendor_root(host: &str) -> Option<&'static str> {
    let host = host.to_ascii_lowercase();
    if host == "dell.com" || host.ends_with(".dell.com") {
        Some("dell.com")
    } else if host == "lenovo.com" || host.ends_with(".lenovo.com") {
        Some("lenovo.com")
    } else if host == "hp.com" || host.ends_with(".hp.com") {
        Some("hp.com")
    } else {
        None
    }
}

fn descendant_display(node: Node<'_, '_>, container: &str) -> Option<String> {
    let container = node
        .descendants()
        .find(|descendant| tag_is(*descendant, container))?;
    container
        .descendants()
        .find(|descendant| tag_is(*descendant, "Display"))
        .and_then(|display| {
            let value = node_text(display);
            (!value.is_empty()).then_some(value)
        })
        .or_else(|| {
            let value = node_text(container);
            (!value.is_empty()).then_some(value)
        })
}

fn descendant_text(node: Node<'_, '_>, tag: &str) -> Option<String> {
    node.descendants()
        .find(|descendant| tag_is(*descendant, tag))
        .and_then(|descendant| {
            let value = node_text(descendant);
            (!value.is_empty()).then_some(value)
        })
}

fn node_text(node: Node<'_, '_>) -> String {
    node.descendants()
        .filter(|descendant| descendant.is_text())
        .filter_map(|descendant| descendant.text())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn node_path_or_text(node: Node<'_, '_>) -> Option<String> {
    attr_ci(node, "path").or_else(|| {
        let value = node_text(node);
        (!value.is_empty()).then_some(value)
    })
}

fn attr_ci(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.attributes()
        .find(|attribute| attribute.name().eq_ignore_ascii_case(name))
        .map(|attribute| attribute.value().trim().to_string())
        .filter(|value| !value.is_empty())
}

fn tag_is(node: Node<'_, '_>, name: &str) -> bool {
    node.is_element() && node.tag_name().name().eq_ignore_ascii_case(name)
}

fn normalize_machine_value(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

fn machine_values_equal(left: &str, right: &str) -> bool {
    normalize_machine_value(left) == normalize_machine_value(right)
}

fn normalize_id(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn is_windows_11_label(value: &str) -> bool {
    let compact = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    compact.contains("windows11") || compact.contains("win11") || compact.starts_with("w11")
}

fn dell_windows_11_applies(supported_os: &[String], machine: &MachineIdentity) -> bool {
    let windows_11 = supported_os
        .iter()
        .filter(|value| is_windows_11_label(value))
        .collect::<Vec<_>>();
    if windows_11.is_empty() {
        return false;
    }
    let explicit_releases = unique_strings(
        windows_11
            .iter()
            .flat_map(|value| windows_release_markers(value))
            .collect(),
    );
    if explicit_releases.is_empty() {
        return true;
    }
    let Some(current) = machine
        .windows_display_version
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    explicit_releases
        .iter()
        .any(|release| release.eq_ignore_ascii_case(current))
}

fn windows_release_markers(value: &str) -> Vec<String> {
    let chars = value.to_ascii_uppercase().chars().collect::<Vec<_>>();
    chars
        .windows(4)
        .filter(|window| {
            window[0].is_ascii_digit()
                && window[1].is_ascii_digit()
                && window[2] == 'H'
                && matches!(window[3], '1' | '2')
        })
        .map(|window| window.iter().collect::<String>())
        .collect()
}

fn normalize_architecture(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    if lower.contains("arm64") || lower.contains("aarch64") {
        "ARM64".into()
    } else if lower.contains("amd64")
        || lower.contains("x64")
        || lower.contains("x86_64")
        || lower.contains("64")
    {
        "x64".into()
    } else if lower.contains("86") || lower.contains("32") {
        "x86".into()
    } else {
        value.trim().to_string()
    }
}

fn clean_hex(value: String) -> String {
    value
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .to_ascii_uppercase()
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for value in values {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_uppercase();
        if seen.insert(key) {
            output.push(trimmed);
        }
    }
    output
}

fn unknown_compatibility() -> CandidateCompatibility {
    CandidateCompatibility {
        state: CompatibilityState::NeedsReview,
        matched_id: None,
        match_kind: None,
        reasons: vec![
            "OEM catalog metadata has not yet been compared with the selected device.".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{
        dell_windows_11_applies, extract_hardware_ids, force_https_hp_url,
        hp_latest_windows_11_release, lenovo_machine_type, normalize_architecture,
        parse_dell_model_catalog, parse_hp_reference, parse_lenovo_catalog_locations,
        parse_lenovo_descriptor, select_hp_windows_11_release, validate_oem_metadata_url,
    };
    use crate::domain::{DriverSourceKind, MachineIdentity};

    fn machine() -> MachineIdentity {
        MachineIdentity {
            manufacturer: Some("Dell Inc.".into()),
            model: Some("Latitude 7450".into()),
            system_sku: Some("0ABC".into()),
            system_family: None,
            baseboard_manufacturer: Some("Dell Inc.".into()),
            baseboard_product: Some("0ABC".into()),
            bios_version: None,
            windows_display_version: Some("24H2".into()),
            windows_build: Some("26100".into()),
        }
    }

    #[test]
    fn hardware_id_scanner_keeps_full_pnp_ids() {
        let ids = extract_hardware_ids(
            "match: PCI\\VEN_10EC&DEV_8125&SUBSYS_012310EC; USB\\VID_0BDA&PID_4853",
        );
        assert!(ids.contains(&"PCI\\VEN_10EC&DEV_8125&SUBSYS_012310EC".into()));
        assert!(ids.contains(&"USB\\VID_0BDA&PID_4853".into()));
    }

    #[test]
    fn dell_component_builds_exact_pci_subsystem_id() {
        let xml = r#"<Manifest><SoftwareComponent path="FOLDER/driver.exe" releaseID="ABC" vendorVersion="1.2.3" dateTime="2026-06-01"><Name><Display>Realtek LAN Driver</Display></Name><ComponentType value="DRVR"/><SupportedSystems><Brand><Model systemID="0ABC" name="Latitude 7450"/></Brand></SupportedSystems><SupportedDevices><Device><PCIInfo vendorID="10EC" deviceID="8125" subVendorID="1028" subDeviceID="0ABC"/></Device></SupportedDevices><SupportedOperatingSystems><OperatingSystem osCode="W11X64" osArch="x64"/></SupportedOperatingSystems><Criticality><Display>Recommended</Display></Criticality><Cryptography><Hash algorithm="SHA256">aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa</Hash></Cryptography></SoftwareComponent></Manifest>"#;
        let candidates =
            parse_dell_model_catalog(xml, "https://dl.dell.com/catalog/model.cab", &machine(), 10)
                .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source, DriverSourceKind::Dell);
        assert!(
            candidates[0]
                .hardware_ids
                .iter()
                .any(|id| id == "PCI\\VEN_10EC&DEV_8125&SUBSYS_0ABC1028")
        );
        assert_eq!(
            candidates[0].expected_sha256.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }

    #[test]
    fn lenovo_descriptor_requires_device_ids() {
        let machine = MachineIdentity {
            manufacturer: Some("LENOVO".into()),
            model: Some("21KH0000US".into()),
            system_sku: Some("LENOVO_MT_21KH_BU_Think_FM_ThinkPad".into()),
            ..Default::default()
        };
        assert_eq!(lenovo_machine_type(&machine).as_deref(), Some("21KH"));
        let xml = r#"<Package id="n3aud15w" name="Realtek Audio Driver" version="6.0.1"><PackageType type="2"/><ReleaseDate>2026-07-01</ReleaseDate><Vendor>Realtek</Vendor><Severity type="2"/><Files><Installer><File><Name>n3aud15w.exe</Name><Size>1000</Size><Checksum type="SHA256">cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc</Checksum></File></Installer></Files><Dependencies><PnPID>HDAUDIO\FUNC_01&amp;VEN_10EC&amp;DEV_0298</PnPID></Dependencies></Package>"#;
        let candidate = parse_lenovo_descriptor(
            xml,
            "https://download.lenovo.com/pccbbs/mobiles/n3aud15w_2_.xml",
            &machine,
            "21KH",
            10,
        )
        .unwrap()
        .unwrap();
        assert_eq!(candidate.source, DriverSourceKind::Lenovo);
        assert!(
            candidate
                .hardware_ids
                .iter()
                .any(|id| id.starts_with("HDAUDIO\\FUNC_01"))
        );
        assert!(
            candidate
                .download_url
                .as_deref()
                .unwrap()
                .ends_with("n3aud15w.exe")
        );
        assert_eq!(
            candidate.expected_sha256.as_deref(),
            Some("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
        );
    }

    #[test]
    fn lenovo_non_driver_packages_are_not_emitted_even_when_they_reference_hardware() {
        let machine = MachineIdentity {
            manufacturer: Some("LENOVO".into()),
            model: Some("21KH0000US".into()),
            system_sku: Some("LENOVO_MT_21KH_BU_Think_FM_ThinkPad".into()),
            ..Default::default()
        };
        let xml = r#"<Package id="utility" name="Hardware Utility" version="1.0"><PackageType type="1"/><Files><Installer><File><Name>utility.exe</Name></File></Installer></Files><Dependencies><PnPID>PCI\VEN_8086&amp;DEV_1234</PnPID></Dependencies></Package>"#;
        assert!(
            parse_lenovo_descriptor(
                xml,
                "https://download.lenovo.com/pccbbs/mobiles/utility_2_.xml",
                &machine,
                "21KH",
                10,
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn hp_reference_maps_device_solution_to_driver() {
        let platform_list = r#"<ImagePal><Platform><SystemID>8B94</SystemID><OS><OSDescription>Windows 11 23H2</OSDescription><OSReleaseIdDisplay>23H2</OSReleaseIdDisplay></OS><OS><OSDescription>Windows 11 24H2</OSDescription><OSReleaseIdDisplay>24H2</OSReleaseIdDisplay></OS></Platform></ImagePal>"#;
        assert_eq!(
            hp_latest_windows_11_release(platform_list, "8B94").as_deref(),
            Some("24H2")
        );
        let hp_machine = MachineIdentity {
            manufacturer: Some("HP".into()),
            model: Some("HP EliteBook".into()),
            baseboard_product: Some("8B94".into()),
            ..Default::default()
        };
        let reference = r#"<ImagePal><Solutions><UpdateInfo id="SP1"><Name>Intel WLAN Driver</Name><Category>Driver</Category><Version>23.1</Version><Vendor>Intel</Vendor><ReleaseType>Recommended</ReleaseType><Url>https://ftp.hp.com/pub/softpaq/sp000001-000500/sp1.exe</Url><SHA256>bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb</SHA256><Size>1234</Size><DateReleased>2026-07-02</DateReleased></UpdateInfo></Solutions><Devices><Device><PNPDeviceID>PCI\VEN_8086&amp;DEV_51F0&amp;SUBSYS_00948086</PNPDeviceID><Solutions><UpdateInfo IdRef="SP1"/></Solutions></Device></Devices></ImagePal>"#;
        let candidates = parse_hp_reference(
            reference,
            "https://hpia.hpcloud.hp.com/ref/8b94/ref.cab",
            &hp_machine,
            "8B94",
            10,
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source, DriverSourceKind::Hp);
        assert_eq!(
            candidates[0].expected_sha256.as_deref(),
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
    }
    #[test]
    fn hp_reference_release_must_match_current_windows_release() {
        let releases = vec!["25H2".into(), "24H2".into(), "23H2".into()];
        let hp_machine = MachineIdentity {
            windows_display_version: Some("24H2".into()),
            ..Default::default()
        };
        assert_eq!(
            select_hp_windows_11_release(&releases, &hp_machine, "8B94").unwrap(),
            "24H2"
        );
    }

    #[test]
    fn hp_reference_release_never_falls_forward_to_a_different_windows_release() {
        let releases = vec!["25H2".into(), "23H2".into()];
        let hp_machine = MachineIdentity {
            windows_display_version: Some("24H2".into()),
            ..Default::default()
        };
        let error = select_hp_windows_11_release(&releases, &hp_machine, "8B94").unwrap_err();
        assert!(error.contains("will not use a different Windows release"));
    }

    #[test]
    fn dell_explicit_windows_10_only_component_is_not_offered_on_windows_11() {
        let xml = r#"<Manifest><SoftwareComponent path="FOLDER/driver.exe" releaseID="OLD" vendorVersion="1.0"><Name><Display>Legacy Driver</Display></Name><ComponentType value="DRVR"/><SupportedSystems><Brand><Model systemID="0ABC" name="Latitude 7450"/></Brand></SupportedSystems><SupportedDevices><Device><PCIInfo vendorID="10EC" deviceID="8125" subVendorID="1028" subDeviceID="0ABC"/></Device></SupportedDevices><SupportedOperatingSystems><OperatingSystem osCode="W10X64" osArch="x64"/></SupportedOperatingSystems></SoftwareComponent></Manifest>"#;
        let candidates =
            parse_dell_model_catalog(xml, "https://dl.dell.com/catalog/model.cab", &machine(), 10)
                .unwrap();
        assert!(candidates.is_empty());
    }

    #[test]
    fn dell_release_specific_metadata_must_match_current_windows_release() {
        let current = machine();
        assert!(dell_windows_11_applies(
            &["Microsoft Windows 11 24H2 64-bit".into()],
            &current
        ));
        assert!(!dell_windows_11_applies(
            &["Microsoft Windows 11 25H2 64-bit".into()],
            &current
        ));
        assert!(dell_windows_11_applies(&["W11X64".into()], &current));
        assert!(!dell_windows_11_applies(&[], &current));
    }

    #[test]
    fn architecture_normalization_does_not_confuse_arm64_with_x64() {
        assert_eq!(normalize_architecture("ARM64"), "ARM64");
        assert_eq!(normalize_architecture("amd64"), "x64");
    }

    #[test]
    fn dell_windows_11_display_metadata_is_accepted() {
        let xml = r#"<Manifest><SoftwareComponent path="FOLDER/driver.exe" releaseID="W11" vendorVersion="2.0"><Name><Display>Network Driver</Display></Name><ComponentType value="DRVR"/><SupportedSystems><Brand><Model systemID="0ABC" name="Latitude 7450"/></Brand></SupportedSystems><SupportedDevices><Device><PCIInfo vendorID="10EC" deviceID="8125" subVendorID="1028" subDeviceID="0ABC"/></Device></SupportedDevices><SupportedOperatingSystems><OperatingSystem osCode="10.0" osArch="x64"><Display>Microsoft Windows 11 64-bit</Display></OperatingSystem></SupportedOperatingSystems></SoftwareComponent></Manifest>"#;
        let candidates =
            parse_dell_model_catalog(xml, "https://dl.dell.com/catalog/model.cab", &machine(), 10)
                .unwrap();
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates[0]
                .supported_os
                .iter()
                .any(|value| value.to_ascii_lowercase().contains("windows 11"))
        );
    }

    #[test]
    fn dell_system_id_can_match_baseboard_product() {
        let mut system = machine();
        system.system_sku = None;
        system.baseboard_product = Some("0ABC".into());
        let xml = r#"<Manifest><SoftwareComponent path="FOLDER/driver.exe" releaseID="BOARD" vendorVersion="2.0"><Name><Display>Board Driver</Display></Name><ComponentType value="DRVR"/><SupportedSystems><Brand><Model systemID="0ABC" name="Different Marketing Name"/></Brand></SupportedSystems><SupportedDevices><Device><PCIInfo vendorID="10EC" deviceID="8125"/></Device></SupportedDevices><SupportedOperatingSystems><OperatingSystem osCode="W11X64" osArch="x64"/></SupportedOperatingSystems></SoftwareComponent></Manifest>"#;
        assert_eq!(
            parse_dell_model_catalog(xml, "https://dl.dell.com/catalog/model.cab", &system, 10,)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn lenovo_catalog_discards_non_lenovo_descriptor_hosts() {
        let xml = r#"<packages><package><location>https://download.lenovo.com/pccbbs/mobiles/good.xml</location></package><package><location>https://example.com/evil.xml</location></package></packages>"#;
        let locations = parse_lenovo_catalog_locations(xml).unwrap();
        assert_eq!(locations.len(), 1);
        assert!(locations[0].contains("download.lenovo.com"));
    }

    #[test]
    fn hp_accepts_legacy_scheme_less_official_urls_but_not_foreign_hosts() {
        assert!(
            force_https_hp_url("ftp.hp.com/pub/softpaq/sp1.exe")
                .unwrap()
                .starts_with("https://ftp.hp.com/")
        );
        assert!(force_https_hp_url("example.com/sp1.exe").is_err());
    }

    #[test]
    fn oem_metadata_urls_are_source_scoped() {
        assert!(
            validate_oem_metadata_url(
                DriverSourceKind::Dell,
                "https://dl.dell.com/catalog/CatalogIndexPC.cab"
            )
            .is_ok()
        );
        assert!(
            validate_oem_metadata_url(
                DriverSourceKind::Dell,
                "https://download.lenovo.com/catalog/21KH_Win11.xml"
            )
            .is_err()
        );
    }
}
