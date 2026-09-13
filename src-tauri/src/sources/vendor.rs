use std::time::Duration;

use reqwest::blocking::Client;
use scraper::{Html, Selector};

use crate::domain::{
    CandidateCompatibility, CompatibilityState, Device, DriverCandidate, DriverSourceKind,
    SignatureStatus,
};

use super::{
    DriverSource,
    http::{get_with_retry, read_text_limited},
};

const USER_AGENT: &str = concat!("DrvMatch/", env!("CARGO_PKG_VERSION"));
const CACHE_SECONDS: i64 = 12 * 60 * 60;
const MAX_VENDOR_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

pub struct VendorSource {
    kind: DriverSourceKind,
    client: Client,
}

impl VendorSource {
    pub fn new(kind: DriverSourceKind) -> Result<Self, String> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(USER_AGENT)
            .build()
            .map_err(|error| format!("Could not initialize the vendor client: {error}"))?;
        Ok(Self { kind, client })
    }

    fn vendor_id(&self) -> &'static str {
        match self.kind {
            DriverSourceKind::Amd => "VEN_1002",
            DriverSourceKind::Nvidia => "VEN_10DE",
            DriverSourceKind::Intel => "VEN_8086",
            _ => "",
        }
    }

    fn supports(&self, device: &Device) -> bool {
        let vendor_match = device
            .hardware_ids
            .iter()
            .any(|id| id.to_ascii_uppercase().contains(self.vendor_id()));
        if !vendor_match {
            return false;
        }

        match self.kind {
            DriverSourceKind::Amd => amd_product_url(&device.friendly_name).is_some(),
            DriverSourceKind::Nvidia => looks_like_display_device(device),
            DriverSourceKind::Intel => intel_product_url(device).is_some(),
            _ => false,
        }
    }

    fn discover_amd(
        &self,
        device: &Device,
        retrieved_at: i64,
    ) -> Result<Vec<DriverCandidate>, String> {
        let Some(url) = amd_product_url(&device.friendly_name) else {
            // Do not guess an AMD platform/chipset page from VEN_1022 alone.
            // A future chipset adapter must identify the actual platform first.
            return Ok(vec![]);
        };
        let group = "GPU display package";
        let body = read_text_limited(
            get_with_retry(|| self.client.get(&url), "AMD support request")?,
            MAX_VENDOR_RESPONSE_BYTES,
            "AMD support response",
        )?;
        parse_amd(&body, &url, group, device, retrieved_at)
    }

    fn discover_nvidia(
        &self,
        device: &Device,
        retrieved_at: i64,
    ) -> Result<Vec<DriverCandidate>, String> {
        let url =
            "https://www.nvidia.com/Download/processFind.aspx?dtcid=1&lang=en-us&lid=1&osid=57";
        let body = read_text_limited(
            get_with_retry(|| self.client.get(url), "NVIDIA driver request")?,
            MAX_VENDOR_RESPONSE_BYTES,
            "NVIDIA driver response",
        )?;
        parse_nvidia(&body, device, retrieved_at)
    }

    fn discover_intel(
        &self,
        device: &Device,
        retrieved_at: i64,
    ) -> Result<Vec<DriverCandidate>, String> {
        let Some((url, group)) = intel_product_url(device) else {
            return Ok(vec![]);
        };
        let body = read_text_limited(
            get_with_retry(|| self.client.get(url), "Intel Download Center request")?,
            MAX_VENDOR_RESPONSE_BYTES,
            "Intel Download Center response",
        )?;
        parse_intel(&body, url, group, device, retrieved_at)
    }
}

impl DriverSource for VendorSource {
    fn kind(&self) -> DriverSourceKind {
        self.kind
    }

    fn cache_namespace(&self) -> &'static str {
        match self.kind {
            DriverSourceKind::Amd => "amd-official-v1",
            DriverSourceKind::Nvidia => "nvidia-official-v1",
            DriverSourceKind::Intel => "intel-official-v1",
            _ => "vendor-official-v1",
        }
    }

    fn cache_key(&self, device: &Device) -> Option<String> {
        self.supports(device).then(|| {
            format!(
                "{}:{}",
                self.vendor_id(),
                device.friendly_name.trim().to_ascii_lowercase()
            )
        })
    }

    fn ttl_seconds(&self) -> i64 {
        CACHE_SECONDS
    }

    fn discover(&self, device: &Device, retrieved_at: i64) -> Result<Vec<DriverCandidate>, String> {
        match self.kind {
            DriverSourceKind::Amd => self.discover_amd(device, retrieved_at),
            DriverSourceKind::Nvidia => self.discover_nvidia(device, retrieved_at),
            DriverSourceKind::Intel => self.discover_intel(device, retrieved_at),
            _ => Ok(vec![]),
        }
    }
}

fn amd_product_url(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let marker = lower.find("rx ")?;
    let model = lower[marker + 3..]
        .split([',', '('])
        .next()?
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>();
    let number = model
        .first()?
        .trim_matches(|c: char| !c.is_ascii_alphanumeric());
    if number.len() < 4 || !number.chars().take(4).all(|c| c.is_ascii_digit()) {
        return None;
    }
    let suffix = model
        .get(1)
        .copied()
        .filter(|value| matches!(*value, "xt" | "xtx" | "gre"));
    let slug = suffix.map_or_else(|| number.to_string(), |suffix| format!("{number}-{suffix}"));
    let series = &number[..1];
    Some(format!(
        "https://www.amd.com/en/support/downloads/drivers.html/graphics/radeon-rx/radeon-rx-{series}000-series/amd-radeon-rx-{slug}.html"
    ))
}

fn parse_amd(
    html: &str,
    details_url: &str,
    package_group: &str,
    device: &Device,
    retrieved_at: i64,
) -> Result<Vec<DriverCandidate>, String> {
    let document = Html::parse_document(html);
    let articles = Selector::parse("article.driver-download-details")
        .map_err(|error| format!("Invalid AMD article selector: {error}"))?;
    let links =
        Selector::parse("a").map_err(|error| format!("Invalid AMD link selector: {error}"))?;
    let paragraphs =
        Selector::parse("p").map_err(|error| format!("Invalid AMD text selector: {error}"))?;
    let headings =
        Selector::parse("h4").map_err(|error| format!("Invalid AMD heading selector: {error}"))?;
    let mut results = Vec::new();
    for (index, article) in document.select(&articles).enumerate() {
        let title = article
            .select(&headings)
            .next()
            .map(clean_text)
            .unwrap_or_default();
        if title.is_empty() || title.to_ascii_lowercase().contains("auto-detect") {
            continue;
        }
        let values = article
            .select(&paragraphs)
            .map(clean_text)
            .collect::<Vec<_>>();
        let revision = values
            .iter()
            .find(|value| value.chars().any(|c| c.is_ascii_digit()))
            .cloned()
            .unwrap_or_default();
        let version = numeric_version(&revision);
        let channel = if revision.to_ascii_lowercase().contains("optional") {
            "Optional"
        } else if revision.to_ascii_lowercase().contains("preview")
            || revision.to_ascii_lowercase().contains("beta")
        {
            "Preview/Beta"
        } else if revision.to_ascii_lowercase().contains("recommended") {
            "WHQL Recommended"
        } else if revision.to_ascii_lowercase().contains("whql") {
            "WHQL"
        } else {
            "Stable"
        };
        let mut download_url = None;
        let mut notes_url = None;
        for link in article.select(&links) {
            if let Some(href) = link.value().attr("href") {
                if href.ends_with(".exe") {
                    download_url = Some(href.to_string());
                } else if href.to_ascii_lowercase().contains("release-notes") {
                    notes_url = Some(absolute_amd_url(href));
                }
            }
        }
        let signature = if channel.to_ascii_lowercase().contains("whql") {
            SignatureStatus::Whql
        } else {
            SignatureStatus::Authenticode
        };
        results.push(candidate(
            format!("amd:{index}:{}", version.as_deref().unwrap_or("package")),
            DriverSourceKind::Amd,
            title,
            version,
            Some(channel.into()),
            values.iter().find(|v| v.contains("202")).cloned(),
            download_url,
            Some(details_url.into()),
            notes_url,
            "AMD installer package",
            package_group,
            signature,
            device,
            retrieved_at,
        ));
    }
    Ok(results)
}

fn parse_nvidia(
    html: &str,
    device: &Device,
    retrieved_at: i64,
) -> Result<Vec<DriverCandidate>, String> {
    let document = Html::parse_document(html);
    let rows = Selector::parse("tr#driverList")
        .map_err(|error| format!("Invalid NVIDIA row selector: {error}"))?;
    let cells =
        Selector::parse("td").map_err(|error| format!("Invalid NVIDIA cell selector: {error}"))?;
    let links =
        Selector::parse("a").map_err(|error| format!("Invalid NVIDIA link selector: {error}"))?;
    let mut results = Vec::new();
    for (index, row) in document.select(&rows).enumerate() {
        let values = row.select(&cells).map(clean_text).collect::<Vec<_>>();
        if values.len() < 3 {
            continue;
        }
        let title = values[0].trim_end_matches("WHQL").trim().to_string();
        let channel = if title.to_ascii_lowercase().contains("studio") {
            "Studio WHQL"
        } else if title.to_ascii_lowercase().contains("hotfix") {
            "Hotfix"
        } else if values[0].to_ascii_lowercase().contains("beta") {
            "Beta"
        } else {
            "Game Ready WHQL"
        };
        let details_url = row
            .select(&links)
            .next()
            .and_then(|link| link.value().attr("href"))
            .map(|href| {
                if href.starts_with("//") {
                    format!("https:{href}")
                } else {
                    href.to_string()
                }
            });
        let signature = if values[0].to_ascii_lowercase().contains("whql") {
            SignatureStatus::Whql
        } else {
            SignatureStatus::Authenticode
        };
        results.push(candidate(
            format!("nvidia:{index}:{}", values[1]),
            DriverSourceKind::Nvidia,
            title,
            Some(values[1].clone()),
            Some(channel.into()),
            Some(values[2].clone()),
            None,
            details_url.clone(),
            details_url,
            "NVIDIA display driver installer",
            "GPU display package",
            signature,
            device,
            retrieved_at,
        ));
    }
    Ok(results)
}

fn intel_product_url(device: &Device) -> Option<(&'static str, &'static str)> {
    let name = device_search_text(device);
    let result = if name.contains("bluetooth") {
        (
            "https://www.intel.com/content/www/us/en/download/18649/intel-wireless-bluetooth-drivers-for-windows-10-and-windows-11.html",
            "Bluetooth package",
        )
    } else if name.contains("wireless") || name.contains("wi-fi") || name.contains("wifi") {
        (
            "https://www.intel.com/content/www/us/en/download/19351/windows-10-and-windows-11-wi-fi-drivers-for-intel-wireless-adapters.html",
            "Wi-Fi package",
        )
    } else if name.contains("ethernet") || name.contains("network") {
        (
            "https://www.intel.com/content/www/us/en/download/15084/intel-ethernet-adapter-complete-driver-pack.html",
            "Ethernet package",
        )
    } else if name.contains("storage") || name.contains("rapid storage") {
        (
            "https://www.intel.com/content/www/us/en/download/15667/intel-rapid-storage-technology-intel-rst-user-interface-and-driver.html",
            "Storage package",
        )
    } else if name.contains("chipset") || name.contains("system") {
        (
            "https://www.intel.com/content/www/us/en/download/19347/chipset-inf-utility.html",
            "Chipset and system package",
        )
    } else if name.contains("display")
        || name.contains("graphics")
        || name.contains("arc")
        || name.contains("iris")
        || name.contains("uhd")
    {
        (
            "https://www.intel.com/content/www/us/en/download/785597/intel-arc-iris-xe-graphics-windows.html",
            "GPU display package",
        )
    } else {
        return None;
    };
    Some(result)
}

fn looks_like_display_device(device: &Device) -> bool {
    let name = device_search_text(device);
    name.contains("display")
        || name.contains("graphics")
        || name.contains("geforce")
        || name.contains("quadro")
        || name.contains("rtx")
        || name.contains("gtx")
}

fn device_search_text(device: &Device) -> String {
    format!(
        "{} {} {}",
        device.friendly_name,
        device.description,
        device.class_name.as_deref().unwrap_or("")
    )
    .to_ascii_lowercase()
}

fn parse_intel(
    html: &str,
    details_url: &str,
    group: &str,
    device: &Device,
    retrieved_at: i64,
) -> Result<Vec<DriverCandidate>, String> {
    let document = Html::parse_document(html);
    let title_selector = Selector::parse("h1")
        .map_err(|error| format!("Invalid Intel heading selector: {error}"))?;
    let title = document
        .select(&title_selector)
        .next()
        .map(clean_text)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Intel response did not contain driver metadata.".to_string())?;
    let text = document.root_element().text().collect::<Vec<_>>().join(" ");
    let version =
        numeric_version_after(&text, "Version").or_else(|| numeric_version_after(&text, "version"));
    let channel = if text.to_ascii_lowercase().contains("recommended") {
        "Recommended"
    } else {
        "Stable"
    };
    Ok(vec![candidate(
        format!("intel:{}", version.as_deref().unwrap_or("package")),
        DriverSourceKind::Intel,
        title,
        version,
        Some(channel.into()),
        None,
        None,
        Some(details_url.into()),
        Some(details_url.into()),
        "Intel installer package",
        group,
        SignatureStatus::Authenticode,
        device,
        retrieved_at,
    )])
}

#[allow(clippy::too_many_arguments)]
fn candidate(
    id: String,
    source: DriverSourceKind,
    title: String,
    version: Option<String>,
    channel: Option<String>,
    publication_date: Option<String>,
    download_url: Option<String>,
    details_url: Option<String>,
    release_notes_url: Option<String>,
    package_type: &str,
    package_group: &str,
    signature: SignatureStatus,
    device: &Device,
    retrieved_at: i64,
) -> DriverCandidate {
    let provider = match source {
        DriverSourceKind::Amd => "AMD",
        DriverSourceKind::Nvidia => "NVIDIA",
        DriverSourceKind::Intel => "Intel",
        _ => "Vendor",
    };
    DriverCandidate {
        id: id.clone(),
        source,
        source_specific_id: id,
        alternate_sources: vec![],
        display_name: title,
        provider: Some(provider.into()),
        manufacturer: Some(provider.into()),
        version,
        version_is_package_version: true,
        driver_date: None,
        publication_date,
        class_name: device.class_name.clone(),
        // Product/download pages are discovery evidence, not proof that the
        // downloadable package contains this device's IDs. Exact applicability
        // is established only after package metadata/INF inspection.
        supported_os: vec![],
        supported_architectures: vec![],
        hardware_ids: vec![],
        compatible_ids: vec![],
        download_url,
        details_url,
        release_notes_url,
        release_channel: channel,
        oem_models: vec![],
        known_issues: vec![],
        known_regressions: vec![],
        fixed_issues: vec![],
        security_relevant: false,
        signature,
        expected_sha256: None,
        package_type: Some(package_type.into()),
        package_group: Some(package_group.into()),
        size_bytes: None,
        retrieved_at,
        compatibility: CandidateCompatibility {
            state: CompatibilityState::NeedsReview,
            matched_id: None,
            match_kind: None,
            reasons: vec![],
        },
    }
}

fn numeric_version(value: &str) -> Option<String> {
    value
        .split_whitespace()
        .find(|token| {
            token.chars().filter(|c| *c == '.').count() >= 1
                && token.chars().any(|c| c.is_ascii_digit())
        })
        .map(|token| {
            token
                .trim_matches(|c: char| !c.is_ascii_digit() && c != '.')
                .into()
        })
}

fn numeric_version_after(value: &str, marker: &str) -> Option<String> {
    let (_, tail) = value.split_once(marker)?;
    numeric_version(tail)
}

fn absolute_amd_url(href: &str) -> String {
    if href.starts_with("http") {
        href.into()
    } else {
        format!("https://www.amd.com{href}")
    }
}

fn clean_text(element: scraper::ElementRef<'_>) -> String {
    element
        .text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{amd_product_url, parse_amd, parse_intel, parse_nvidia};
    use crate::domain::{Device, DeviceCondition, DriverSourceKind};

    fn device(name: &str, id: &str) -> Device {
        Device {
            instance_id: id.into(),
            friendly_name: name.into(),
            description: name.into(),
            manufacturer: None,
            class_name: Some("Display".into()),
            class_guid: None,
            hardware_ids: vec![id.into()],
            compatible_ids: vec![],
            present: true,
            problem_code: None,
            problem_status: None,
            condition: DeviceCondition::Current,
            installed_driver: None,
            hardware_identity: None,
        }
    }

    #[test]
    fn routes_amd_radeon_products_to_an_official_product_page() {
        let rx_7600 = amd_product_url("AMD Radeon RX 7600").unwrap();
        assert!(rx_7600.contains("radeon-rx-7000-series"));
        assert!(rx_7600.ends_with("amd-radeon-rx-7600.html"));
        assert!(
            amd_product_url("AMD Radeon RX 7900 XTX")
                .unwrap()
                .ends_with("amd-radeon-rx-7900-xtx.html")
        );
    }

    #[test]
    fn parses_official_vendor_fixtures() {
        let amd = parse_amd(
            include_str!("fixtures/amd_driver.html"),
            "https://www.amd.com/example",
            "GPU display package",
            &device("AMD Radeon RX 7600", "PCI\\VEN_1002&DEV_7480"),
            1,
        )
        .unwrap();
        assert_eq!(amd.len(), 2);
        assert_eq!(amd[0].release_channel.as_deref(), Some("WHQL Recommended"));
        assert!(amd[0].release_notes_url.is_some());
        assert!(amd[0].hardware_ids.is_empty());
        assert!(amd[0].compatible_ids.is_empty());

        let nvidia = parse_nvidia(
            include_str!("fixtures/nvidia_driver.html"),
            &device("NVIDIA GeForce RTX 4070", "PCI\\VEN_10DE&DEV_2786"),
            1,
        )
        .unwrap();
        assert_eq!(nvidia.len(), 2);
        assert_eq!(nvidia[1].release_channel.as_deref(), Some("Studio WHQL"));

        let intel = parse_intel(
            include_str!("fixtures/intel_driver.html"),
            "https://www.intel.com/example",
            "Wi-Fi package",
            &device("Intel Wi-Fi 6E AX211", "PCI\\VEN_8086&DEV_7A70"),
            1,
        )
        .unwrap();
        assert_eq!(intel[0].version.as_deref(), Some("24.70.0"));
        assert_eq!(intel[0].source, DriverSourceKind::Intel);
    }
}
