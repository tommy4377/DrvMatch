mod microsoft_catalog;
mod windows_update;

use crate::{
    domain::{
        CandidateCompatibility, CandidateDiscovery, CompatibilityState, Device, DriverCandidate,
        DriverSourceKind, MatchKind, SourceHealth, SourceHealthState,
    },
    metadata_cache::{MetadataCache, unix_timestamp},
};

use self::{microsoft_catalog::MicrosoftCatalogSource, windows_update::WindowsUpdateSource};

const WINDOWS_UPDATE_CACHE_SECONDS: i64 = 15 * 60;
const CATALOG_CACHE_SECONDS: i64 = 24 * 60 * 60;

pub trait DriverSource {
    fn kind(&self) -> DriverSourceKind;
    fn cache_namespace(&self) -> &'static str;
    fn cache_key(&self, device: &Device) -> Option<String>;
    fn ttl_seconds(&self) -> i64;
    fn discover(&self, device: &Device, retrieved_at: i64) -> Result<Vec<DriverCandidate>, String>;
}

pub fn discover_microsoft_candidates(device: &Device, cache: &MetadataCache) -> CandidateDiscovery {
    let checked_at = unix_timestamp();
    let mut candidates = Vec::new();
    let mut sources = Vec::new();

    collect_source(
        &WindowsUpdateSource,
        device,
        cache,
        checked_at,
        &mut candidates,
        &mut sources,
    );

    match MicrosoftCatalogSource::new() {
        Ok(catalog) => collect_source(
            &catalog,
            device,
            cache,
            checked_at,
            &mut candidates,
            &mut sources,
        ),
        Err(error) => sources.push(SourceHealth {
            source: DriverSourceKind::MicrosoftCatalog,
            state: SourceHealthState::Failed,
            checked_at,
            cached: false,
            candidate_count: 0,
            message: Some(error),
        }),
    }

    let (candidates, rejected_candidates) = candidates
        .into_iter()
        .map(|mut candidate| {
            candidate.compatibility = evaluate_compatibility(&candidate, device);
            candidate
        })
        .partition(|candidate| candidate.compatibility.state != CompatibilityState::Incompatible);

    CandidateDiscovery {
        device_instance_id: device.instance_id.clone(),
        checked_at,
        candidates,
        rejected_candidates,
        sources,
    }
}

pub fn resolve_catalog_download(
    update_id: &str,
    cache: &MetadataCache,
) -> Result<crate::domain::DownloadResolution, String> {
    microsoft_catalog::resolve_download(update_id, cache)
}

fn collect_source(
    source: &dyn DriverSource,
    device: &Device,
    cache: &MetadataCache,
    checked_at: i64,
    candidates: &mut Vec<DriverCandidate>,
    sources: &mut Vec<SourceHealth>,
) {
    let Some(cache_key) = source.cache_key(device) else {
        sources.push(SourceHealth {
            source: source.kind(),
            state: SourceHealthState::Skipped,
            checked_at,
            cached: false,
            candidate_count: 0,
            message: Some("No hardware ID is available for this source query.".into()),
        });
        return;
    };

    match cache.get::<Vec<DriverCandidate>>(source.cache_namespace(), &cache_key) {
        Ok(Some(cached)) => {
            let count = cached.value.len();
            candidates.extend(cached.value);
            sources.push(SourceHealth {
                source: source.kind(),
                state: SourceHealthState::Available,
                checked_at: cached.fetched_at,
                cached: true,
                candidate_count: count,
                message: None,
            });
        }
        Ok(None) => match source.discover(device, checked_at) {
            Ok(discovered) => {
                let count = discovered.len();
                let cache_message = cache
                    .put(
                        source.cache_namespace(),
                        &cache_key,
                        source.ttl_seconds(),
                        &discovered,
                    )
                    .err();
                candidates.extend(discovered);
                sources.push(SourceHealth {
                    source: source.kind(),
                    state: SourceHealthState::Available,
                    checked_at,
                    cached: false,
                    candidate_count: count,
                    message: cache_message,
                });
            }
            Err(error) => sources.push(SourceHealth {
                source: source.kind(),
                state: SourceHealthState::Failed,
                checked_at,
                cached: false,
                candidate_count: 0,
                message: Some(error),
            }),
        },
        Err(error) => sources.push(SourceHealth {
            source: source.kind(),
            state: SourceHealthState::Failed,
            checked_at,
            cached: false,
            candidate_count: 0,
            message: Some(error),
        }),
    }
}

fn evaluate_compatibility(candidate: &DriverCandidate, device: &Device) -> CandidateCompatibility {
    let device_hardware_ids = normalized_ids(&device.hardware_ids);
    let device_compatible_ids = normalized_ids(&device.compatible_ids);

    if let Some(matched_id) = candidate.hardware_ids.iter().find_map(|id| {
        let normalized = normalize_id(id);
        device_hardware_ids
            .contains(&normalized)
            .then_some(normalized)
    }) {
        return source_match(candidate.source, matched_id, MatchKind::ExactHardwareId);
    }

    if let Some(matched_id) = candidate
        .hardware_ids
        .iter()
        .chain(&candidate.compatible_ids)
        .find_map(|id| {
            let normalized = normalize_id(id);
            device_compatible_ids
                .contains(&normalized)
                .then_some(normalized)
        })
    {
        return source_match(candidate.source, matched_id, MatchKind::CompatibleId);
    }

    CandidateCompatibility {
        state: CompatibilityState::Incompatible,
        matched_id: None,
        match_kind: None,
        reasons: vec!["No hardware or compatible ID matches the selected device.".into()],
    }
}

fn source_match(
    source: DriverSourceKind,
    matched_id: String,
    match_kind: MatchKind,
) -> CandidateCompatibility {
    let (state, source_reason) = match source {
        DriverSourceKind::WindowsUpdate => (
            CompatibilityState::Compatible,
            "Windows Update reports this package as applicable to the current machine.",
        ),
        DriverSourceKind::MicrosoftCatalog => (
            CompatibilityState::NeedsReview,
            "The Catalog returned this package for the exact ID; OS and architecture still require package inspection.",
        ),
    };
    CandidateCompatibility {
        state,
        matched_id: Some(matched_id),
        match_kind: Some(match_kind),
        reasons: vec![
            match match_kind {
                MatchKind::ExactHardwareId => "Exact hardware ID match.",
                MatchKind::CompatibleId => "Compatible ID match.",
                MatchKind::WindowsApplicable => "Applicable to the current Windows installation.",
            }
            .into(),
            source_reason.into(),
        ],
    }
}

fn normalized_ids(ids: &[String]) -> std::collections::HashSet<String> {
    ids.iter().map(|id| normalize_id(id)).collect()
}

fn normalize_id(id: &str) -> String {
    id.trim().to_ascii_uppercase()
}

impl WindowsUpdateSource {
    const CACHE_SECONDS: i64 = WINDOWS_UPDATE_CACHE_SECONDS;
}

impl MicrosoftCatalogSource {
    const CACHE_SECONDS: i64 = CATALOG_CACHE_SECONDS;
}

#[cfg(test)]
mod tests {
    use super::evaluate_compatibility;
    use crate::domain::{
        CandidateCompatibility, CompatibilityState, Device, DeviceCondition, DriverCandidate,
        DriverSourceKind, MatchKind, SignatureStatus,
    };

    fn device() -> Device {
        Device {
            instance_id: "PCI\\VEN_1234&DEV_5678".into(),
            friendly_name: "Fixture".into(),
            description: "Fixture".into(),
            manufacturer: None,
            class_name: None,
            class_guid: None,
            hardware_ids: vec!["PCI\\VEN_1234&DEV_5678&SUBSYS_00000001".into()],
            compatible_ids: vec!["PCI\\VEN_1234&DEV_5678".into()],
            present: true,
            problem_code: None,
            problem_status: None,
            condition: DeviceCondition::Current,
            installed_driver: None,
        }
    }

    fn candidate(source: DriverSourceKind, hardware_id: &str) -> DriverCandidate {
        DriverCandidate {
            id: "candidate".into(),
            source,
            source_specific_id: "source-id".into(),
            display_name: "Fixture driver".into(),
            provider: None,
            manufacturer: None,
            version: None,
            driver_date: None,
            publication_date: None,
            class_name: None,
            supported_os: vec![],
            supported_architectures: vec![],
            hardware_ids: vec![hardware_id.into()],
            compatible_ids: vec![],
            download_url: None,
            details_url: None,
            release_notes_url: None,
            release_channel: None,
            signature: SignatureStatus::Unknown,
            package_type: None,
            size_bytes: None,
            retrieved_at: 0,
            compatibility: CandidateCompatibility {
                state: CompatibilityState::NeedsReview,
                matched_id: None,
                match_kind: None,
                reasons: vec![],
            },
        }
    }

    #[test]
    fn exact_windows_update_match_is_compatible() {
        let compatibility = evaluate_compatibility(
            &candidate(
                DriverSourceKind::WindowsUpdate,
                "pci\\ven_1234&dev_5678&subsys_00000001",
            ),
            &device(),
        );
        assert_eq!(compatibility.state, CompatibilityState::Compatible);
        assert_eq!(compatibility.match_kind, Some(MatchKind::ExactHardwareId));
    }

    #[test]
    fn catalog_match_requires_package_review() {
        let compatibility = evaluate_compatibility(
            &candidate(
                DriverSourceKind::MicrosoftCatalog,
                "PCI\\VEN_1234&DEV_5678&SUBSYS_00000001",
            ),
            &device(),
        );
        assert_eq!(compatibility.state, CompatibilityState::NeedsReview);
    }

    #[test]
    fn unrelated_candidate_is_rejected_with_reason() {
        let compatibility = evaluate_compatibility(
            &candidate(DriverSourceKind::WindowsUpdate, "PCI\\VEN_ABCD&DEV_EF01"),
            &device(),
        );
        assert_eq!(compatibility.state, CompatibilityState::Incompatible);
        assert!(!compatibility.reasons.is_empty());
    }
}
