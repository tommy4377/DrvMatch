use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
}
