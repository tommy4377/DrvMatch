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
