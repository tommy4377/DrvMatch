use std::time::Duration;

use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};

use crate::{
    domain::{
        CandidateCompatibility, CompatibilityState, Device, DownloadResolution, DriverCandidate,
        DriverSourceKind, SignatureStatus,
    },
    metadata_cache::MetadataCache,
};

use super::DriverSource;

const CATALOG_ORIGIN: &str = "https://www.catalog.update.microsoft.com";
const USER_AGENT: &str = concat!("DrvMatch/", env!("CARGO_PKG_VERSION"));
const DOWNLOAD_CACHE_SECONDS: i64 = 24 * 60 * 60;
const MAX_RESULTS: usize = 50;

pub struct MicrosoftCatalogSource {
    client: Client,
}

impl MicrosoftCatalogSource {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(USER_AGENT)
            .build()
            .map_err(|error| format!("Could not initialize the Catalog client: {error}"))?;
        Ok(Self { client })
    }

    fn search(&self, hardware_id: &str, retrieved_at: i64) -> Result<Vec<DriverCandidate>, String> {
        let response = self
            .client
            .get(format!("{CATALOG_ORIGIN}/Search.aspx"))
            .query(&[("q", format!("\"{hardware_id}\""))])
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| format!("Microsoft Update Catalog search failed: {error}"))?;
        let html = response
            .text()
            .map_err(|error| format!("Could not read the Catalog response: {error}"))?;
        parse_search_results(&html, hardware_id, retrieved_at)
    }
}

impl DriverSource for MicrosoftCatalogSource {
    fn kind(&self) -> DriverSourceKind {
        DriverSourceKind::MicrosoftCatalog
    }

    fn cache_namespace(&self) -> &'static str {
        "microsoft-catalog-search-v1"
    }

    fn cache_key(&self, device: &Device) -> Option<String> {
        device
            .hardware_ids
            .iter()
            .find(|id| !id.trim().is_empty())
            .map(|id| id.trim().to_ascii_uppercase())
    }

    fn ttl_seconds(&self) -> i64 {
        Self::CACHE_SECONDS
    }

    fn discover(&self, device: &Device, retrieved_at: i64) -> Result<Vec<DriverCandidate>, String> {
        let hardware_id = self
            .cache_key(device)
            .ok_or_else(|| "No hardware ID is available for a Catalog query.".to_string())?;
        self.search(&hardware_id, retrieved_at)
    }
}

pub fn resolve_download(
    update_id: &str,
    cache: &MetadataCache,
) -> Result<DownloadResolution, String> {
    validate_update_id(update_id)?;
    let namespace = "microsoft-catalog-download-v1";
    if let Some(cached) = cache.get::<String>(namespace, update_id)? {
        return Ok(DownloadResolution {
            source: DriverSourceKind::MicrosoftCatalog,
            source_specific_id: update_id.into(),
            download_url: cached.value,
            resolved_at: cached.fetched_at,
            cached: true,
        });
    }

    let source = MicrosoftCatalogSource::new()?;
    let payload = format!(
        r#"[{{"size":0,"languages":"","uidInfo":"{update_id}","updateID":"{update_id}"}}]"#
    );
    let response = source
        .client
        .post(format!("{CATALOG_ORIGIN}/DownloadDialog.aspx"))
        .form(&[("updateIDs", payload)])
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("Catalog download metadata request failed: {error}"))?;
    let body = response
        .text()
        .map_err(|error| format!("Could not read Catalog download metadata: {error}"))?;
    let download_url = parse_download_url(&body)
        .ok_or_else(|| "The Catalog did not provide a package download URL.".to_string())?;
    let resolved_at = cache.put(namespace, update_id, DOWNLOAD_CACHE_SECONDS, &download_url)?;
    Ok(DownloadResolution {
        source: DriverSourceKind::MicrosoftCatalog,
        source_specific_id: update_id.into(),
        download_url,
        resolved_at,
        cached: false,
    })
}

fn parse_search_results(
    html: &str,
    queried_hardware_id: &str,
    retrieved_at: i64,
) -> Result<Vec<DriverCandidate>, String> {
    let document = Html::parse_document(html);
    let rows = Selector::parse("#ctl00_catalogBody_updateMatches tr")
        .map_err(|error| format!("Invalid Catalog row selector: {error}"))?;
    let cells =
        Selector::parse("td").map_err(|error| format!("Invalid Catalog cell selector: {error}"))?;
    let detail_link = Selector::parse("a[onclick*='goToDetails']")
        .map_err(|error| format!("Invalid Catalog link selector: {error}"))?;
    let original_size = Selector::parse("[id$='_originalSize']")
        .map_err(|error| format!("Invalid Catalog size selector: {error}"))?;

    let mut candidates = Vec::new();
    for row in document.select(&rows) {
        let row_cells = row.select(&cells).collect::<Vec<_>>();
        if row_cells.len() < 7 {
            continue;
        }
        let Some(link) = row.select(&detail_link).next() else {
            continue;
        };
        let Some(update_id) = extract_update_id(&link) else {
            continue;
        };
        let display_name = clean_text(link);
        if display_name.is_empty() {
            continue;
        }
        let products = non_empty(clean_text(row_cells[2])).into_iter().collect();
        let publication_date = non_empty(clean_text(row_cells[4]));
        let version = non_empty(clean_text(row_cells[5]));
        let size_bytes = row_cells[6]
            .select(&original_size)
            .next()
            .and_then(|value| clean_text(value).parse::<u64>().ok());
        let provider = display_name
            .split(" - ")
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        candidates.push(DriverCandidate {
            id: format!("catalog:{update_id}"),
            source: DriverSourceKind::MicrosoftCatalog,
            source_specific_id: update_id.clone(),
            display_name,
            provider: provider.clone(),
            manufacturer: provider,
            version,
            driver_date: None,
            publication_date,
            class_name: non_empty(clean_text(row_cells[3])),
            supported_os: products,
            supported_architectures: vec![],
            hardware_ids: vec![queried_hardware_id.into()],
            compatible_ids: vec![],
            download_url: None,
            details_url: Some(format!(
                "{CATALOG_ORIGIN}/ScopedViewInline.aspx?updateid={update_id}"
            )),
            release_notes_url: None,
            release_channel: None,
            oem_models: vec![],
            known_issues: vec![],
            known_regressions: vec![],
            fixed_issues: vec![],
            security_relevant: false,
            signature: SignatureStatus::Unknown,
            package_type: Some("Microsoft Update Catalog package".into()),
            size_bytes,
            retrieved_at,
            compatibility: CandidateCompatibility {
                state: CompatibilityState::NeedsReview,
                matched_id: None,
                match_kind: None,
                reasons: vec![],
            },
        });
        if candidates.len() == MAX_RESULTS {
            break;
        }
    }
    Ok(candidates)
}

fn extract_update_id(link: &ElementRef<'_>) -> Option<String> {
    let onclick = link.value().attr("onclick")?;
    let value = onclick.split_once("goToDetails(\"")?.1.split_once("\")")?.0;
    validate_update_id(value).ok()?;
    Some(value.to_ascii_lowercase())
}

fn validate_update_id(update_id: &str) -> Result<(), String> {
    let valid = update_id.len() == 36
        && update_id.chars().enumerate().all(|(index, value)| {
            if [8, 13, 18, 23].contains(&index) {
                value == '-'
            } else {
                value.is_ascii_hexdigit()
            }
        });
    valid
        .then_some(())
        .ok_or_else(|| "The Catalog update ID is invalid.".to_string())
}

fn parse_download_url(html: &str) -> Option<String> {
    html.split(".url = '")
        .skip(1)
        .filter_map(|remainder| remainder.split_once('\'').map(|(url, _)| url))
        .find(|url| url.starts_with("https://") || url.starts_with("http://"))
        .map(str::to_owned)
}

fn clean_text(element: ElementRef<'_>) -> String {
    element
        .text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::{MicrosoftCatalogSource, parse_download_url, parse_search_results};

    #[test]
    fn parses_catalog_search_fixture() {
        let candidates = parse_search_results(
            include_str!("fixtures/catalog_search.html"),
            "PCI\\VEN_10DE&DEV_1C82",
            123,
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(
            candidate.source_specific_id,
            "9ea16e47-10e5-4315-887c-4c80280f5b2f"
        );
        assert_eq!(candidate.version.as_deref(), Some("32.0.15.8097"));
        assert_eq!(candidate.size_bytes, Some(1_153_117_685));
        assert_eq!(candidate.hardware_ids, vec!["PCI\\VEN_10DE&DEV_1C82"]);
    }

    #[test]
    fn parses_download_metadata_fixture() {
        assert_eq!(
            parse_download_url(include_str!("fixtures/catalog_download.html")).as_deref(),
            Some("https://catalog.s.download.windowsupdate.com/example/driver.cab")
        );
    }

    #[test]
    #[ignore = "requires the live Microsoft Update Catalog"]
    fn live_catalog_query_returns_driver_metadata() {
        let source = MicrosoftCatalogSource::new().unwrap();
        let candidates = source.search("PCI\\VEN_10DE&DEV_1C82", 123).unwrap();
        assert!(!candidates.is_empty());
    }
}
