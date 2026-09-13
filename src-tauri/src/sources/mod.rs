mod microsoft_catalog;
mod oem;
mod vendor;
mod windows_update;

use crate::{
    domain::{
        CandidateCompatibility, CandidateDiscovery, CompatibilityState, Device, DriverCandidate,
        DriverSourceKind, MachineIdentity, MatchKind, SourceHealth, SourceHealthState,
    },
    metadata_cache::{MetadataCache, unix_timestamp},
};

use self::vendor::VendorSource;
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

pub fn discover_candidates(
    device: &Device,
    machine: &MachineIdentity,
    cache: &MetadataCache,
    enabled_sources: &[DriverSourceKind],
) -> CandidateDiscovery {
    let checked_at = unix_timestamp();
    let mut candidates = Vec::new();
    let mut sources = Vec::new();

    if enabled_sources.contains(&DriverSourceKind::WindowsUpdate) {
        collect_source(
            &WindowsUpdateSource,
            device,
            cache,
            checked_at,
            &mut candidates,
            &mut sources,
        );
    } else {
        sources.push(disabled_source(DriverSourceKind::WindowsUpdate, checked_at));
    }

    if !enabled_sources.contains(&DriverSourceKind::MicrosoftCatalog) {
        sources.push(disabled_source(
            DriverSourceKind::MicrosoftCatalog,
            checked_at,
        ));
    } else {
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
    }

    for kind in [
        DriverSourceKind::Amd,
        DriverSourceKind::Nvidia,
        DriverSourceKind::Intel,
    ] {
        if !enabled_sources.contains(&kind) {
            sources.push(disabled_source(kind, checked_at));
            continue;
        }
        match VendorSource::new(kind) {
            Ok(source) => collect_source(
                &source,
                device,
                cache,
                checked_at,
                &mut candidates,
                &mut sources,
            ),
            Err(error) => sources.push(SourceHealth {
                source: kind,
                state: SourceHealthState::Failed,
                checked_at,
                cached: false,
                candidate_count: 0,
                message: Some(error),
            }),
        }
    }

    for kind in [
        DriverSourceKind::Dell,
        DriverSourceKind::Lenovo,
        DriverSourceKind::Hp,
    ] {
        if !enabled_sources.contains(&kind) {
            sources.push(disabled_source(kind, checked_at));
            continue;
        }
        let (mut discovered, health) = oem::collect(kind, machine, device, cache, checked_at);
        candidates.append(&mut discovered);
        sources.push(health);
    }

    let (candidates, rejected_candidates): (Vec<DriverCandidate>, Vec<DriverCandidate>) =
        reconcile_candidates(candidates)
            .into_iter()
            .map(|mut candidate| {
                candidate.compatibility = evaluate_compatibility(&candidate, device);
                crate::ranking::apply_hard_compatibility(&mut candidate);
                candidate
            })
            .partition(|candidate| {
                candidate.compatibility.state != CompatibilityState::Incompatible
            });

    let recommendation =
        crate::ranking::rank_candidates(device, candidates.clone(), rejected_candidates.clone());

    CandidateDiscovery {
        device_instance_id: device.instance_id.clone(),
        checked_at,
        candidates,
        rejected_candidates,
        sources,
        recommendation,
    }
}

fn disabled_source(source: DriverSourceKind, checked_at: i64) -> SourceHealth {
    SourceHealth {
        source,
        state: SourceHealthState::Skipped,
        checked_at,
        cached: false,
        candidate_count: 0,
        message: Some("Disabled in Settings.".into()),
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
    if candidate.hardware_ids.is_empty() && candidate.compatible_ids.is_empty() {
        let vendor_reason = match candidate.source {
            DriverSourceKind::Amd => Some(
                "AMD product-page metadata was discovered, but the downloaded package has not yet been inspected for this device's exact IDs.",
            ),
            DriverSourceKind::Nvidia => Some(
                "NVIDIA channel metadata was discovered, but exact product/package applicability has not yet been proven.",
            ),
            DriverSourceKind::Intel => Some(
                "Intel family metadata was discovered, but the package supported-products/INF data has not yet been inspected.",
            ),
            DriverSourceKind::Dell | DriverSourceKind::Lenovo | DriverSourceKind::Hp => Some(
                "OEM model metadata alone is not enough to prove that this package contains a matching driver for the selected device.",
            ),
            _ => None,
        };
        if let Some(reason) = vendor_reason {
            return CandidateCompatibility {
                state: CompatibilityState::NeedsReview,
                matched_id: None,
                match_kind: None,
                reasons: vec![reason.into()],
            };
        }
    }

    let device_hardware_ids = normalized_ids(&device.hardware_ids);
    let device_compatible_ids = normalized_ids(&device.compatible_ids);

    if let Some(matched_id) = candidate.hardware_ids.iter().find_map(|id| {
        let normalized = normalize_id(id);
        device_hardware_ids
            .contains(&normalized)
            .then_some(normalized)
    }) {
        return source_match(candidate, matched_id, MatchKind::ExactHardwareId);
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
        return source_match(candidate, matched_id, MatchKind::CompatibleId);
    }

    CandidateCompatibility {
        state: CompatibilityState::Incompatible,
        matched_id: None,
        match_kind: None,
        reasons: vec!["No hardware or compatible ID matches the selected device.".into()],
    }
}

fn source_match(
    candidate: &DriverCandidate,
    matched_id: String,
    match_kind: MatchKind,
) -> CandidateCompatibility {
    let (state, source_reason) = match candidate.source {
        DriverSourceKind::WindowsUpdate => (
            CompatibilityState::Compatible,
            "Windows Update reports this package as applicable to the current machine.",
        ),
        DriverSourceKind::MicrosoftCatalog => (
            CompatibilityState::NeedsReview,
            "The Catalog returned this package for the exact ID; OS and architecture still require package inspection.",
        ),
        DriverSourceKind::Amd => (
            CompatibilityState::NeedsReview,
            "AMD publishes this package for the discovered product family; package-level applicability still requires verification.",
        ),
        DriverSourceKind::Nvidia => (
            CompatibilityState::NeedsReview,
            "NVIDIA publishes this channel for GeForce hardware; product support must be confirmed before installation.",
        ),
        DriverSourceKind::Intel => (
            CompatibilityState::NeedsReview,
            "Intel publishes this package for the detected device family; the supported-products list must be confirmed before installation.",
        ),
        DriverSourceKind::Dell | DriverSourceKind::Lenovo | DriverSourceKind::Hp => {
            if candidate.oem_models.is_empty() {
                (
                    CompatibilityState::NeedsReview,
                    "The OEM package has a device-ID match, but exact machine applicability was not recorded.",
                )
            } else {
                (
                    CompatibilityState::Compatible,
                    "The official OEM catalog associates this package with the detected machine and provides a matching device ID.",
                )
            }
        }
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
                MatchKind::ExactOemModel => "Exact OEM machine-model applicability.",
            }
            .into(),
            source_reason.into(),
        ],
    }
}

fn reconcile_candidates(candidates: Vec<DriverCandidate>) -> Vec<DriverCandidate> {
    let mut reconciled: Vec<DriverCandidate> = Vec::new();
    for mut candidate in candidates {
        let duplicate = reconciled.iter().position(|existing| {
            let family = provider_family(existing);
            existing.version.is_some()
                && existing.version == candidate.version
                && !family.is_empty()
                && family == provider_family(&candidate)
                && ids_overlap(existing, &candidate)
        });
        if let Some(index) = duplicate {
            let existing = &mut reconciled[index];
            if metadata_richness(&candidate) > metadata_richness(existing) {
                candidate.alternate_sources.push(existing.source);
                candidate
                    .alternate_sources
                    .extend(existing.alternate_sources.iter().copied());
                candidate
                    .alternate_sources
                    .sort_by_key(|source| format!("{source:?}"));
                candidate.alternate_sources.dedup();
                *existing = candidate;
            } else if existing.source != candidate.source
                && !existing.alternate_sources.contains(&candidate.source)
            {
                existing.alternate_sources.push(candidate.source);
            }
        } else {
            reconciled.push(candidate);
        }
    }
    reconciled
}

fn provider_family(candidate: &DriverCandidate) -> String {
    let value = candidate
        .provider
        .as_deref()
        .or(candidate.manufacturer.as_deref())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if value.contains("amd") || value.contains("advanced micro devices") {
        return "amd".into();
    }
    for family in ["nvidia", "intel", "microsoft"] {
        if value.contains(family) {
            return family.into();
        }
    }
    value
}

fn ids_overlap(left: &DriverCandidate, right: &DriverCandidate) -> bool {
    left.hardware_ids.iter().any(|left_id| {
        right
            .hardware_ids
            .iter()
            .any(|right_id| normalize_id(left_id) == normalize_id(right_id))
    })
}

fn metadata_richness(candidate: &DriverCandidate) -> usize {
    [
        candidate.download_url.is_some(),
        candidate.details_url.is_some(),
        candidate.release_notes_url.is_some(),
        candidate.release_channel.is_some(),
        candidate.package_group.is_some(),
        candidate.size_bytes.is_some(),
        candidate.expected_sha256.is_some(),
        !candidate.oem_models.is_empty(),
        !candidate.supported_os.is_empty(),
        !candidate.supported_architectures.is_empty(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count()
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
    use super::{evaluate_compatibility, reconcile_candidates};
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
            hardware_identity: None,
        }
    }

    fn candidate(source: DriverSourceKind, hardware_id: &str) -> DriverCandidate {
        DriverCandidate {
            id: "candidate".into(),
            source,
            source_specific_id: "source-id".into(),
            alternate_sources: vec![],
            display_name: "Fixture driver".into(),
            provider: None,
            manufacturer: None,
            version: None,
            version_is_package_version: false,
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
            oem_models: vec![],
            known_issues: vec![],
            known_regressions: vec![],
            fixed_issues: vec![],
            security_relevant: false,
            signature: SignatureStatus::Unknown,
            expected_sha256: None,
            package_type: None,
            package_group: None,
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
    fn vendor_discovery_without_package_ids_needs_review() {
        let mut vendor = candidate(DriverSourceKind::Amd, "PCI\\VEN_1002&DEV_7480");
        vendor.hardware_ids.clear();
        vendor.compatible_ids.clear();
        let compatibility = evaluate_compatibility(&vendor, &device());
        assert_eq!(compatibility.state, CompatibilityState::NeedsReview);
        assert!(compatibility.matched_id.is_none());
        assert!(compatibility.match_kind.is_none());
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

    #[test]
    fn duplicate_packages_keep_richer_metadata_and_source_provenance() {
        let mut catalog = candidate(DriverSourceKind::MicrosoftCatalog, "PCI\\VEN_1002&DEV_7480");
        catalog.version = Some("32.0.21001.9028".into());
        catalog.provider = Some("AMD".into());
        let mut amd = candidate(DriverSourceKind::Amd, "PCI\\VEN_1002&DEV_7480");
        amd.version = catalog.version.clone();
        amd.version_is_package_version = true;
        amd.provider = Some("AMD".into());
        amd.download_url = Some("https://drivers.amd.com/example.exe".into());
        amd.release_notes_url = Some("https://www.amd.com/release-notes".into());
        amd.release_channel = Some("WHQL Recommended".into());

        let reconciled = reconcile_candidates(vec![catalog, amd]);
        assert_eq!(reconciled.len(), 1);
        assert_eq!(reconciled[0].source, DriverSourceKind::Amd);
        assert_eq!(
            reconciled[0].alternate_sources,
            vec![DriverSourceKind::MicrosoftCatalog]
        );
    }
    #[test]
    fn exact_oem_catalog_device_match_is_installable_only_with_model_evidence() {
        let mut oem = candidate(
            DriverSourceKind::Dell,
            "PCI\\VEN_1234&DEV_5678&SUBSYS_00000001",
        );
        oem.oem_models = vec!["Latitude Fixture".into()];
        let compatibility = evaluate_compatibility(&oem, &device());
        assert_eq!(compatibility.state, CompatibilityState::Compatible);
        assert_eq!(compatibility.match_kind, Some(MatchKind::ExactHardwareId));

        oem.oem_models.clear();
        let without_model = evaluate_compatibility(&oem, &device());
        assert_eq!(without_model.state, CompatibilityState::NeedsReview);
    }
}
