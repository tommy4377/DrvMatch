use std::cmp::Ordering;

use crate::domain::{
    CandidateCompatibility, CompatibilityState, Device, DriverCandidate, DriverRecommendation,
    DriverSourceKind, MatchKind, RankFactor, RankedCandidate, RecommendationState, SignatureStatus,
};

const RECOMMENDATION_MARGIN: i32 = 8;

#[derive(Clone, Copy, Debug)]
pub struct RankingContext<'a> {
    pub architecture: &'a str,
}

impl RankingContext<'static> {
    pub fn current() -> Self {
        Self {
            architecture: std::env::consts::ARCH,
        }
    }
}

pub fn rank_candidates(
    device: &Device,
    candidates: Vec<DriverCandidate>,
    mut rejected: Vec<DriverCandidate>,
) -> DriverRecommendation {
    rank_with_context(device, candidates, &mut rejected, RankingContext::current())
}

pub fn apply_hard_compatibility(candidate: &mut DriverCandidate) {
    if let Some(reason) = hard_rejection(candidate, RankingContext::current()) {
        candidate.compatibility = incompatible(reason);
    }
}

fn rank_with_context(
    device: &Device,
    candidates: Vec<DriverCandidate>,
    rejected: &mut Vec<DriverCandidate>,
    context: RankingContext<'_>,
) -> DriverRecommendation {
    let mut scored = Vec::new();
    for mut candidate in candidates {
        if let Some(reason) = hard_rejection(&candidate, context) {
            candidate.compatibility = incompatible(reason);
            rejected.push(candidate);
            continue;
        }
        let factors = candidate_factors(device, &candidate);
        let score = factors.iter().map(|factor| factor.score).sum();
        scored.push((candidate, score, factors));
    }

    scored.sort_by(compare_ranked);
    let (current_score, current_factors) = current_factors(device)
        .map(|factors| {
            (
                Some(factors.iter().map(|factor| factor.score).sum()),
                factors,
            )
        })
        .unwrap_or((None, Vec::new()));

    let best = scored.first();
    let state = recommendation_state(device, best, current_score);
    let best_is_recommendable = best.is_some_and(|entry| {
        entry.0.compatibility.state == CompatibilityState::Compatible
            && !entry
                .2
                .iter()
                .any(|factor| factor.key == "known-regression")
    });
    let selected_candidate_id = (matches!(
        state,
        RecommendationState::Recommended | RecommendationState::Missing
    ) && best_is_recommendable)
        .then(|| best.map(|entry| entry.0.id.clone()))
        .flatten();
    let summary = recommendation_summary(device, best, current_score, state, best_is_recommendable);
    let newest_not_best = newest_not_best(&scored);

    let ranked_candidates = scored
        .into_iter()
        .map(|(candidate, score, factors)| {
            let candidate_state = if selected_candidate_id.as_deref() == Some(candidate.id.as_str())
            {
                RecommendationState::Recommended
            } else if candidate.compatibility.state == CompatibilityState::NeedsReview
                || factors
                    .iter()
                    .any(|factor| factor.key == "known-regression")
            {
                RecommendationState::NotRecommended
            } else {
                RecommendationState::Optional
            };
            let summary = candidate_summary(&candidate, &factors, candidate_state);
            RankedCandidate {
                candidate,
                score,
                state: candidate_state,
                factors,
                summary,
            }
        })
        .chain(rejected.drain(..).map(|candidate| RankedCandidate {
            summary: candidate.compatibility.reasons.join(" "),
            candidate,
            score: i32::MIN,
            state: RecommendationState::NotRecommended,
            factors: Vec::new(),
        }))
        .collect();

    DriverRecommendation {
        state,
        selected_candidate_id,
        current_score,
        current_factors,
        summary,
        newest_not_best,
        ranked_candidates,
    }
}

fn hard_rejection(candidate: &DriverCandidate, context: RankingContext<'_>) -> Option<String> {
    if candidate.compatibility.state == CompatibilityState::Incompatible {
        return Some(
            candidate
                .compatibility
                .reasons
                .first()
                .cloned()
                .unwrap_or_else(|| "The package is not compatible with this device.".into()),
        );
    }
    if candidate.signature == SignatureStatus::Unsigned {
        return Some("Unsigned driver packages are not eligible for recommendation.".into());
    }
    if !candidate.supported_architectures.is_empty()
        && !candidate
            .supported_architectures
            .iter()
            .any(|architecture| architecture_matches(architecture, context.architecture))
    {
        return Some(format!(
            "Package architecture does not support Windows {}.",
            display_architecture(context.architecture)
        ));
    }
    None
}

fn candidate_factors(device: &Device, candidate: &DriverCandidate) -> Vec<RankFactor> {
    let mut factors = Vec::new();
    let match_score = match candidate.compatibility.match_kind {
        Some(MatchKind::ExactHardwareId)
            if candidate
                .compatibility
                .matched_id
                .as_deref()
                .is_some_and(has_subsystem) =>
        {
            60
        }
        Some(MatchKind::ExactHardwareId) => 48,
        Some(MatchKind::WindowsApplicable) => 36,
        Some(MatchKind::CompatibleId) => 25,
        None => 0,
    };
    factors.push(factor(
        "match-specificity",
        "Hardware match",
        match_score,
        match match_score {
            60 => "Exact hardware and subsystem ID match.",
            48 => "Exact hardware ID match.",
            36 => "Windows reports the package as applicable.",
            25 => "Compatible ID match; less specific than an exact hardware ID.",
            _ => "No match specificity evidence is available.",
        },
    ));

    if provider_matches_device(device, candidate) || !candidate.oem_models.is_empty() {
        factors.push(factor(
            "oem-applicability",
            "OEM applicability",
            16,
            "Provider or model metadata aligns with this machine.",
        ));
    }

    factors.push(factor(
        "source-trust",
        "Source trust",
        match candidate.source {
            DriverSourceKind::WindowsUpdate => 18,
            DriverSourceKind::MicrosoftCatalog => 14,
        },
        match candidate.source {
            DriverSourceKind::WindowsUpdate => {
                "Windows Update supplied applicability metadata for this machine."
            }
            DriverSourceKind::MicrosoftCatalog => {
                "Microsoft Update Catalog is an official package source."
            }
        },
    ));

    let (signature_score, signature_detail) = signature_factor(candidate.signature);
    factors.push(factor(
        "signature",
        "Signing",
        signature_score,
        signature_detail,
    ));

    if let Some(channel) = candidate.release_channel.as_deref() {
        let normalized = channel.to_ascii_lowercase();
        let (score, detail) = if normalized.contains("recommended")
            || normalized.contains("stable")
            || normalized.contains("production")
            || normalized.contains("whql")
        {
            (14, "Stable or recommended release channel.")
        } else if normalized.contains("beta") || normalized.contains("preview") {
            (-18, "Preview or beta channel is not preferred by default.")
        } else if normalized.contains("optional") {
            (
                0,
                "Optional release channel provides no default stability preference.",
            )
        } else {
            (
                0,
                "Release channel is reported but has no calibrated preference.",
            )
        };
        factors.push(factor("release-channel", "Release channel", score, detail));
    }

    if candidate.compatibility.state == CompatibilityState::NeedsReview {
        factors.push(factor(
            "package-review",
            "Package inspection",
            -12,
            "OS and architecture applicability still require package inspection.",
        ));
    }
    if !candidate.known_issues.is_empty() {
        factors.push(factor(
            "known-issues",
            "Known issues",
            -8,
            "Reliable source metadata reports known issues for this package.",
        ));
    }
    if !candidate.known_regressions.is_empty() {
        factors.push(factor(
            "known-regression",
            "Known regression",
            -80,
            "A known regression prevents a normal recommendation.",
        ));
    }
    if !candidate.fixed_issues.is_empty() {
        factors.push(factor(
            "known-fix",
            "Relevant fixes",
            4,
            "Source metadata identifies fixes in this package.",
        ));
    }
    if candidate.security_relevant {
        factors.push(factor(
            "security",
            "Security relevance",
            15,
            "A reliable source identifies a security-relevant fix.",
        ));
    }
    if let (Some(installed_version), Some(candidate_version)) = (
        device
            .installed_driver
            .as_ref()
            .and_then(|installed| installed.version.as_deref()),
        candidate.version.as_deref(),
    ) && let Some(ordering) = compare_versions(candidate_version, installed_version)
    {
        let (score, detail) = match ordering {
            Ordering::Greater => (
                5,
                "Newer than the installed version; recency has limited weight.",
            ),
            Ordering::Equal => (0, "Same version as the installed package."),
            Ordering::Less => (
                -2,
                "Older than the installed version; stronger suitability can still win.",
            ),
        };
        factors.push(factor("recency", "Version relation", score, detail));
    }
    factors
}

fn current_factors(device: &Device) -> Option<Vec<RankFactor>> {
    let installed = device.installed_driver.as_ref()?;
    let matching_id = installed.matching_id.as_deref().unwrap_or_default();
    let normalized = normalize_id(matching_id);
    let exact = device
        .hardware_ids
        .iter()
        .any(|id| normalize_id(id) == normalized);
    let compatible = device
        .compatible_ids
        .iter()
        .any(|id| normalize_id(id) == normalized);
    let (match_score, match_detail) = if exact && has_subsystem(matching_id) {
        (
            60,
            "Installed package matches the exact hardware and subsystem ID.",
        )
    } else if exact {
        (48, "Installed package matches an exact hardware ID.")
    } else if compatible {
        (25, "Installed package uses a compatible ID match.")
    } else {
        (
            20,
            "Windows currently associates this package with the device.",
        )
    };
    let mut factors = vec![
        factor(
            "match-specificity",
            "Hardware match",
            match_score,
            match_detail,
        ),
        factor(
            "installed-applicability",
            "Installed applicability",
            16,
            "The package is installed and Windows reports the device as present.",
        ),
    ];
    let (score, detail) = signature_factor(installed.signature);
    factors.push(factor("signature", "Signing", score, detail));
    if provider_text_matches(
        device.manufacturer.as_deref(),
        installed.provider.as_deref(),
    ) {
        factors.push(factor(
            "oem-applicability",
            "OEM applicability",
            16,
            "Installed provider metadata aligns with the device manufacturer.",
        ));
    }
    if installed.generic_microsoft {
        factors.push(factor(
            "generic-package",
            "Generic package",
            -12,
            "The installed package identifies itself as a generic Microsoft driver.",
        ));
    }
    Some(factors)
}

fn recommendation_state(
    device: &Device,
    best: Option<&(DriverCandidate, i32, Vec<RankFactor>)>,
    current_score: Option<i32>,
) -> RecommendationState {
    let Some((candidate, score, factors)) = best else {
        return if device.installed_driver.is_some() {
            RecommendationState::Current
        } else {
            RecommendationState::Missing
        };
    };
    if device.installed_driver.is_none() {
        return RecommendationState::Missing;
    }
    if candidate.compatibility.state == CompatibilityState::NeedsReview
        || factors
            .iter()
            .any(|factor| factor.key == "known-regression")
    {
        return RecommendationState::Current;
    }
    let current = current_score.unwrap_or_default();
    if *score >= current + RECOMMENDATION_MARGIN {
        RecommendationState::Recommended
    } else if *score > current {
        RecommendationState::Optional
    } else {
        RecommendationState::Current
    }
}

fn recommendation_summary(
    device: &Device,
    best: Option<&(DriverCandidate, i32, Vec<RankFactor>)>,
    current_score: Option<i32>,
    state: RecommendationState,
    best_is_recommendable: bool,
) -> String {
    match state {
        RecommendationState::Recommended => format!(
            "{} is meaningfully better suited to this machine than the installed package.",
            best.map(|entry| entry.0.display_name.as_str()).unwrap_or("The leading candidate")
        ),
        RecommendationState::Optional => "A valid alternative scores slightly higher, but the evidence is not strong enough to recommend changing drivers.".into(),
        RecommendationState::Current => {
            if best.is_none() {
                "No suitable alternative was found; the installed driver remains current.".into()
            } else if best.is_some_and(|entry| entry.0.compatibility.state == CompatibilityState::NeedsReview) {
                "The installed driver remains current because the leading Catalog package still needs OS and architecture inspection.".into()
            } else if best.is_some_and(|entry| entry.2.iter().any(|factor| factor.key == "known-regression")) {
                "The installed driver remains current because the leading candidate carries a known regression.".into()
            } else {
                let _ = current_score;
                "The installed driver remains the better match after comparing hardware specificity, OEM applicability, trust, signing, and release stability.".into()
            }
        }
        RecommendationState::Missing => {
            if let Some((candidate, _, _)) = best.filter(|_| best_is_recommendable) {
                format!("No driver is installed; {} is the best compatible candidate.", candidate.display_name)
            } else if best.is_some() {
                "No driver is installed, but available candidates require review or carry a disqualifying penalty.".into()
            } else {
                "No driver is installed and no compatible candidate was found.".into()
            }
        }
        RecommendationState::NotRecommended => {
            let _ = device;
            "Available candidates are not suitable for normal installation.".into()
        }
    }
}

fn candidate_summary(
    candidate: &DriverCandidate,
    factors: &[RankFactor],
    state: RecommendationState,
) -> String {
    if state == RecommendationState::NotRecommended {
        return factors
            .iter()
            .find(|factor| factor.score < 0)
            .map(|factor| factor.detail.clone())
            .unwrap_or_else(|| "This candidate requires additional compatibility review.".into());
    }
    let strongest = factors
        .iter()
        .filter(|factor| factor.score > 0)
        .max_by_key(|factor| factor.score)
        .map(|factor| factor.detail.as_str())
        .unwrap_or("Compatible alternative.");
    format!("{} {}", candidate.display_name, strongest)
}

fn newest_not_best(scored: &[(DriverCandidate, i32, Vec<RankFactor>)]) -> Option<String> {
    let best = scored.first()?;
    let newest = scored.iter().max_by(|left, right| {
        compare_optional_versions(left.0.version.as_deref(), right.0.version.as_deref())
    })?;
    if newest.0.id == best.0.id {
        return None;
    }
    let mut advantages: Vec<(&str, i32)> = best
        .2
        .iter()
        .filter_map(|factor| {
            let competing_score = newest
                .2
                .iter()
                .find(|other| other.key == factor.key)
                .map_or(0, |other| other.score);
            (factor.score > competing_score)
                .then_some((factor.label.as_str(), factor.score - competing_score))
        })
        .collect();
    advantages.sort_by_key(|(_, difference)| std::cmp::Reverse(*difference));
    let evidence = advantages
        .iter()
        .take(2)
        .map(|(label, _)| label.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" and ");
    let evidence = if evidence.is_empty() {
        "combined suitability".into()
    } else {
        evidence
    };
    Some(format!(
        "{} is newer, but {} ranks higher because its {} evidence outweighs recency.",
        newest.0.display_name, best.0.display_name, evidence
    ))
}

fn compare_ranked(
    left: &(DriverCandidate, i32, Vec<RankFactor>),
    right: &(DriverCandidate, i32, Vec<RankFactor>),
) -> Ordering {
    right
        .1
        .cmp(&left.1)
        .then_with(|| match_specificity(&right.0).cmp(&match_specificity(&left.0)))
        .then_with(|| {
            compare_optional_versions(right.0.version.as_deref(), left.0.version.as_deref())
        })
        .then_with(|| left.0.id.cmp(&right.0.id))
}

fn match_specificity(candidate: &DriverCandidate) -> i32 {
    match candidate.compatibility.match_kind {
        Some(MatchKind::ExactHardwareId)
            if candidate
                .compatibility
                .matched_id
                .as_deref()
                .is_some_and(has_subsystem) =>
        {
            3
        }
        Some(MatchKind::ExactHardwareId) => 2,
        Some(MatchKind::WindowsApplicable) => 1,
        Some(MatchKind::CompatibleId) | None => 0,
    }
}

fn signature_factor(signature: SignatureStatus) -> (i32, &'static str) {
    match signature {
        SignatureStatus::Whql => (18, "WHQL signature provides strong signing evidence."),
        SignatureStatus::Inbox => (16, "Windows inbox package is signed and trusted."),
        SignatureStatus::Authenticode => (12, "Authenticode signature is present."),
        SignatureStatus::SignedUnclassified => {
            (8, "Package is signed; classification is not reported.")
        }
        SignatureStatus::Unknown => (0, "Signature status is not yet known."),
        SignatureStatus::Unsigned => (-100, "Unsigned packages are blocked."),
    }
}

fn factor(key: &str, label: &str, score: i32, detail: &str) -> RankFactor {
    RankFactor {
        key: key.into(),
        label: label.into(),
        score,
        detail: detail.into(),
    }
}

fn provider_matches_device(device: &Device, candidate: &DriverCandidate) -> bool {
    provider_text_matches(
        device.manufacturer.as_deref(),
        candidate
            .provider
            .as_deref()
            .or(candidate.manufacturer.as_deref()),
    )
}

fn provider_text_matches(left: Option<&str>, right: Option<&str>) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    let left = left.to_ascii_lowercase();
    let right = right.to_ascii_lowercase();
    left.split_whitespace()
        .filter(|token| token.len() >= 3)
        .any(|token| right.contains(token))
}

fn architecture_matches(reported: &str, current: &str) -> bool {
    let reported = reported.to_ascii_lowercase();
    match current {
        "x86_64" => {
            reported.contains("x64") || reported.contains("amd64") || reported.contains("x86_64")
        }
        "aarch64" => reported.contains("arm64") || reported.contains("aarch64"),
        "x86" => reported == "x86" || reported.contains("32-bit"),
        other => reported.contains(other),
    }
}

fn display_architecture(architecture: &str) -> &str {
    match architecture {
        "x86_64" => "x64",
        "aarch64" => "ARM64",
        "x86" => "x86",
        other => other,
    }
}

fn has_subsystem(id: &str) -> bool {
    id.to_ascii_uppercase().contains("SUBSYS_")
}

fn normalize_id(id: &str) -> String {
    id.trim().to_ascii_uppercase()
}

fn compare_optional_versions(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => {
            compare_versions(left, right).unwrap_or_else(|| left.cmp(right))
        }
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    let parse = |value: &str| {
        value
            .split(|character: char| !character.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()
    };
    let mut left = parse(left)?;
    let mut right = parse(right)?;
    let length = left.len().max(right.len());
    left.resize(length, 0);
    right.resize(length, 0);
    Some(left.cmp(&right))
}

fn incompatible(reason: String) -> CandidateCompatibility {
    CandidateCompatibility {
        state: CompatibilityState::Incompatible,
        matched_id: None,
        match_kind: None,
        reasons: vec![reason],
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{RankingContext, rank_with_context};
    use crate::domain::{
        CandidateCompatibility, CompatibilityState, Device, DeviceCondition, DriverCandidate,
        DriverSourceKind, InstalledDriver, MatchKind, RecommendationState, SignatureStatus,
    };

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Scenario {
        name: String,
        installed: Option<FixtureInstalled>,
        candidates: Vec<FixtureCandidate>,
        expected_state: RecommendationState,
        expected_selected: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureInstalled {
        matching_id: String,
        version: String,
        provider: String,
        signature: SignatureStatus,
        generic_microsoft: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureCandidate {
        id: String,
        matched_id: String,
        match_kind: MatchKind,
        version: String,
        channel: Option<String>,
        architectures: Vec<String>,
        signature: SignatureStatus,
        known_regression: bool,
        #[serde(default)]
        provider: Option<String>,
    }

    fn device(installed: Option<FixtureInstalled>) -> Device {
        Device {
            instance_id: "PCI\\VEN_10EC&DEV_8125&SUBSYS_012310EC".into(),
            friendly_name: "Realtek PCIe 2.5GbE Family Controller".into(),
            description: "Network adapter fixture".into(),
            manufacturer: Some("Realtek".into()),
            class_name: Some("Net".into()),
            class_guid: None,
            hardware_ids: vec!["PCI\\VEN_10EC&DEV_8125&SUBSYS_012310EC".into()],
            compatible_ids: vec!["PCI\\VEN_10EC&DEV_8125".into()],
            present: true,
            problem_code: None,
            problem_status: None,
            condition: if installed.is_some() {
                DeviceCondition::Current
            } else {
                DeviceCondition::Missing
            },
            installed_driver: installed.map(|installed| InstalledDriver {
                description: Some("Fixture installed driver".into()),
                provider: Some(installed.provider),
                version: Some(installed.version),
                driver_date: None,
                inf_path: None,
                published_inf_name: None,
                inf_section: None,
                matching_id: Some(installed.matching_id),
                driver_key: None,
                driver_rank: None,
                signer: None,
                catalog_file: None,
                signature: installed.signature,
                inf_signature_verified: true,
                generic_microsoft: installed.generic_microsoft,
            }),
        }
    }

    fn candidate(fixture: FixtureCandidate) -> DriverCandidate {
        DriverCandidate {
            id: fixture.id.clone(),
            source: DriverSourceKind::WindowsUpdate,
            source_specific_id: fixture.id,
            display_name: "Fixture candidate".into(),
            provider: Some(fixture.provider.clone().unwrap_or_else(|| "Realtek".into())),
            manufacturer: Some(fixture.provider.unwrap_or_else(|| "Realtek".into())),
            version: Some(fixture.version),
            driver_date: None,
            publication_date: None,
            class_name: Some("Net".into()),
            supported_os: vec!["Windows 11".into()],
            supported_architectures: fixture.architectures,
            hardware_ids: vec![fixture.matched_id.clone()],
            compatible_ids: vec![],
            download_url: None,
            details_url: None,
            release_notes_url: None,
            release_channel: fixture.channel,
            oem_models: vec![],
            known_issues: vec![],
            known_regressions: fixture
                .known_regression
                .then(|| "Fixture regression".into())
                .into_iter()
                .collect(),
            fixed_issues: vec![],
            security_relevant: false,
            signature: fixture.signature,
            package_type: Some("Fixture package".into()),
            size_bytes: None,
            retrieved_at: 0,
            compatibility: CandidateCompatibility {
                state: CompatibilityState::Compatible,
                matched_id: Some(fixture.matched_id),
                match_kind: Some(fixture.match_kind),
                reasons: vec!["Fixture compatibility evidence.".into()],
            },
        }
    }

    #[test]
    fn normalized_fixture_scenarios_cover_core_ranking_contract() {
        let scenarios: Vec<Scenario> =
            serde_json::from_str(include_str!("fixtures/ranking_scenarios.json")).unwrap();
        assert!(scenarios.len() >= 6);

        for scenario in scenarios {
            let device = device(scenario.installed);
            let candidates = scenario.candidates.into_iter().map(candidate).collect();
            let mut rejected = vec![];
            let result = rank_with_context(
                &device,
                candidates,
                &mut rejected,
                RankingContext {
                    architecture: "x86_64",
                },
            );
            assert_eq!(result.state, scenario.expected_state, "{}", scenario.name);
            assert_eq!(
                result.selected_candidate_id, scenario.expected_selected,
                "{}",
                scenario.name
            );
        }
    }

    #[test]
    fn ranking_is_deterministic_and_explanations_come_from_factors() {
        let scenarios: Vec<Scenario> =
            serde_json::from_str(include_str!("fixtures/ranking_scenarios.json")).unwrap();
        let scenario = scenarios.into_iter().next().unwrap();
        let device = device(scenario.installed);
        let candidates: Vec<_> = scenario.candidates.into_iter().map(candidate).collect();
        let mut first_rejected = vec![];
        let mut second_rejected = vec![];
        let first = rank_with_context(
            &device,
            candidates.clone(),
            &mut first_rejected,
            RankingContext {
                architecture: "x86_64",
            },
        );
        let second = rank_with_context(
            &device,
            candidates,
            &mut second_rejected,
            RankingContext {
                architecture: "x86_64",
            },
        );
        assert_eq!(first, second);
        assert!(
            first.ranked_candidates[0]
                .factors
                .iter()
                .any(|factor| factor.score > 0)
        );
        assert!(!first.ranked_candidates[0].summary.is_empty());
        assert!(first.newest_not_best.is_some());
    }

    #[test]
    fn incompatible_architecture_is_excluded_and_explained() {
        let fixture = FixtureCandidate {
            id: "arm-only".into(),
            matched_id: "PCI\\VEN_10EC&DEV_8125&SUBSYS_012310EC".into(),
            match_kind: MatchKind::ExactHardwareId,
            version: "2.0".into(),
            channel: Some("Recommended".into()),
            architectures: vec!["ARM64".into()],
            signature: SignatureStatus::Whql,
            known_regression: false,
            provider: None,
        };
        let mut rejected = vec![];
        let result = rank_with_context(
            &device(None),
            vec![candidate(fixture)],
            &mut rejected,
            RankingContext {
                architecture: "x86_64",
            },
        );
        let rejected = &result.ranked_candidates[0];
        assert_eq!(rejected.state, RecommendationState::NotRecommended);
        assert!(rejected.summary.contains("architecture"));
    }
}
