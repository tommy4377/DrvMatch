use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub instance_id: String,
    pub friendly_name: String,
    pub description: String,
    pub manufacturer: Option<String>,
    pub class_name: Option<String>,
    pub class_guid: Option<String>,
    pub hardware_ids: Vec<String>,
    pub compatible_ids: Vec<String>,
    pub present: bool,
    pub problem_code: Option<u32>,
    pub problem_status: Option<i32>,
    pub condition: DeviceCondition,
    pub installed_driver: Option<InstalledDriver>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledDriver {
    pub description: Option<String>,
    pub provider: Option<String>,
    pub version: Option<String>,
    pub driver_date: Option<i64>,
    pub inf_path: Option<String>,
    pub published_inf_name: Option<String>,
    pub inf_section: Option<String>,
    pub matching_id: Option<String>,
    pub driver_key: Option<String>,
    pub driver_rank: Option<u32>,
    pub signer: Option<String>,
    pub catalog_file: Option<String>,
    pub signature: SignatureStatus,
    pub inf_signature_verified: bool,
    pub generic_microsoft: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceCondition {
    Current,
    Missing,
    Problem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignatureStatus {
    Whql,
    Inbox,
    Authenticode,
    SignedUnclassified,
    Unsigned,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub id: i64,
    pub scanned_at: i64,
    pub device_count: usize,
    pub problem_count: usize,
    pub missing_count: usize,
    pub generic_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySnapshot {
    pub summary: ScanSummary,
    pub devices: Vec<Device>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DriverSourceKind {
    WindowsUpdate,
    MicrosoftCatalog,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceHealthState {
    Available,
    Failed,
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealth {
    pub source: DriverSourceKind,
    pub state: SourceHealthState,
    pub checked_at: i64,
    pub cached: bool,
    pub candidate_count: usize,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CompatibilityState {
    Compatible,
    NeedsReview,
    Incompatible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MatchKind {
    ExactHardwareId,
    CompatibleId,
    WindowsApplicable,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateCompatibility {
    pub state: CompatibilityState,
    pub matched_id: Option<String>,
    pub match_kind: Option<MatchKind>,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverCandidate {
    pub id: String,
    pub source: DriverSourceKind,
    pub source_specific_id: String,
    pub display_name: String,
    pub provider: Option<String>,
    pub manufacturer: Option<String>,
    pub version: Option<String>,
    pub driver_date: Option<i64>,
    pub publication_date: Option<String>,
    pub class_name: Option<String>,
    pub supported_os: Vec<String>,
    pub supported_architectures: Vec<String>,
    pub hardware_ids: Vec<String>,
    pub compatible_ids: Vec<String>,
    pub download_url: Option<String>,
    pub details_url: Option<String>,
    pub release_notes_url: Option<String>,
    pub release_channel: Option<String>,
    #[serde(default)]
    pub oem_models: Vec<String>,
    #[serde(default)]
    pub known_issues: Vec<String>,
    #[serde(default)]
    pub known_regressions: Vec<String>,
    #[serde(default)]
    pub fixed_issues: Vec<String>,
    #[serde(default)]
    pub security_relevant: bool,
    pub signature: SignatureStatus,
    pub package_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub retrieved_at: i64,
    pub compatibility: CandidateCompatibility,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDiscovery {
    pub device_instance_id: String,
    pub checked_at: i64,
    pub candidates: Vec<DriverCandidate>,
    pub rejected_candidates: Vec<DriverCandidate>,
    pub sources: Vec<SourceHealth>,
    pub recommendation: DriverRecommendation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecommendationState {
    Recommended,
    Optional,
    Current,
    Missing,
    NotRecommended,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankFactor {
    pub key: String,
    pub label: String,
    pub score: i32,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedCandidate {
    pub candidate: DriverCandidate,
    pub score: i32,
    pub state: RecommendationState,
    pub factors: Vec<RankFactor>,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverRecommendation {
    pub state: RecommendationState,
    pub selected_candidate_id: Option<String>,
    pub current_score: Option<i32>,
    pub current_factors: Vec<RankFactor>,
    pub summary: String,
    pub newest_not_best: Option<String>,
    pub ranked_candidates: Vec<RankedCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResolution {
    pub source: DriverSourceKind,
    pub source_specific_id: String,
    pub download_url: String,
    pub resolved_at: i64,
    pub cached: bool,
}
