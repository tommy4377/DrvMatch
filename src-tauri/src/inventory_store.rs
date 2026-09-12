use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{Device, DeviceCondition, InventorySnapshot, ScanSummary};

#[derive(Clone, Debug)]
pub struct InventoryStore {
    path: PathBuf,
}

impl InventoryStore {
    pub fn open(app_data_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(app_data_dir)
            .map_err(|error| format!("Could not create the DrvMatch data directory: {error}"))?;
        let store = Self {
            path: app_data_dir.join("inventory.sqlite3"),
        };
        let connection = store.connection()?;
        initialize(&connection)?;
        Ok(store)
    }

    fn connection(&self) -> Result<Connection, String> {
        Connection::open(&self.path).map_err(database_error)
    }

    pub fn save_scan(&self, devices: Vec<Device>) -> Result<InventorySnapshot, String> {
        let mut connection = self.connection()?;
        initialize(&connection)?;
        save_scan_to_connection(&mut connection, devices)
    }

    pub fn list_scans(&self) -> Result<Vec<ScanSummary>, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        list_scans_from_connection(&connection)
    }

    pub fn load_scan(&self, id: i64) -> Result<Option<InventorySnapshot>, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        load_scan_from_connection(&connection, id)
    }
}

fn initialize(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         CREATE TABLE IF NOT EXISTS scans (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           scanned_at INTEGER NOT NULL,
           device_count INTEGER NOT NULL,
           problem_count INTEGER NOT NULL,
           missing_count INTEGER NOT NULL,
           generic_count INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS scan_devices (
           scan_id INTEGER NOT NULL REFERENCES scans(id) ON DELETE CASCADE,
           ordinal INTEGER NOT NULL,
           instance_id TEXT NOT NULL,
           payload_json TEXT NOT NULL,
           PRIMARY KEY (scan_id, ordinal)
         );
         CREATE INDEX IF NOT EXISTS scan_devices_instance
           ON scan_devices(scan_id, instance_id);",
        )
        .map_err(database_error)
}

fn save_scan_to_connection(
    connection: &mut Connection,
    devices: Vec<Device>,
) -> Result<InventorySnapshot, String> {
    let scanned_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("System clock is before the Unix epoch: {error}"))?
        .as_secs() as i64;
    let problem_count = devices
        .iter()
        .filter(|device| device.condition == DeviceCondition::Problem)
        .count();
    let missing_count = devices
        .iter()
        .filter(|device| device.condition == DeviceCondition::Missing)
        .count();
    let generic_count = devices
        .iter()
        .filter(|device| {
            device
                .installed_driver
                .as_ref()
                .is_some_and(|driver| driver.generic_microsoft)
        })
        .count();
    let transaction = connection.transaction().map_err(database_error)?;
    transaction.execute(
        "INSERT INTO scans(scanned_at, device_count, problem_count, missing_count, generic_count)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![scanned_at, devices.len() as i64, problem_count as i64, missing_count as i64, generic_count as i64],
    ).map_err(database_error)?;
    let id = transaction.last_insert_rowid();
    {
        let mut statement = transaction.prepare(
            "INSERT INTO scan_devices(scan_id, ordinal, instance_id, payload_json) VALUES (?1, ?2, ?3, ?4)"
        ).map_err(database_error)?;
        for (ordinal, device) in devices.iter().enumerate() {
            let payload = serde_json::to_string(device)
                .map_err(|error| format!("Could not serialize device inventory: {error}"))?;
            statement
                .execute(params![id, ordinal as i64, &device.instance_id, payload])
                .map_err(database_error)?;
        }
    }
    transaction.commit().map_err(database_error)?;
    Ok(InventorySnapshot {
        summary: ScanSummary {
            id,
            scanned_at,
            device_count: devices.len(),
            problem_count,
            missing_count,
            generic_count,
        },
        devices,
    })
}

fn list_scans_from_connection(connection: &Connection) -> Result<Vec<ScanSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, scanned_at, device_count, problem_count, missing_count, generic_count
         FROM scans ORDER BY id DESC LIMIT 100",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], scan_summary_from_row)
        .map_err(database_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
}

fn load_scan_from_connection(
    connection: &Connection,
    id: i64,
) -> Result<Option<InventorySnapshot>, String> {
    let summary = connection.query_row(
        "SELECT id, scanned_at, device_count, problem_count, missing_count, generic_count FROM scans WHERE id = ?1",
        [id],
        scan_summary_from_row,
    ).optional().map_err(database_error)?;
    let Some(summary) = summary else {
        return Ok(None);
    };
    let mut statement = connection
        .prepare("SELECT payload_json FROM scan_devices WHERE scan_id = ?1 ORDER BY ordinal")
        .map_err(database_error)?;
    let rows = statement
        .query_map([id], |row| row.get::<_, String>(0))
        .map_err(database_error)?;
    let mut devices = Vec::with_capacity(summary.device_count);
    for payload in rows {
        let payload = payload.map_err(database_error)?;
        devices.push(
            serde_json::from_str(&payload)
                .map_err(|error| format!("Stored device inventory is invalid: {error}"))?,
        );
    }
    Ok(Some(InventorySnapshot { summary, devices }))
}

fn scan_summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScanSummary> {
    Ok(ScanSummary {
        id: row.get(0)?,
        scanned_at: row.get(1)?,
        device_count: row.get::<_, i64>(2)? as usize,
        problem_count: row.get::<_, i64>(3)? as usize,
        missing_count: row.get::<_, i64>(4)? as usize,
        generic_count: row.get::<_, i64>(5)? as usize,
    })
}

fn database_error(error: rusqlite::Error) -> String {
    format!("Inventory database error: {error}")
}

#[cfg(test)]
mod tests {
    use super::{
        initialize, list_scans_from_connection, load_scan_from_connection, save_scan_to_connection,
    };
    use crate::domain::{Device, DeviceCondition};
    use rusqlite::Connection;

    #[test]
    fn scan_round_trips_through_sqlite() {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize(&connection).unwrap();
        let device = Device {
            instance_id: "PCI\\VEN_1234&DEV_5678".into(),
            friendly_name: "Fixture device".into(),
            description: "Fixture device".into(),
            manufacturer: Some("Fixture vendor".into()),
            class_name: Some("System".into()),
            class_guid: None,
            hardware_ids: vec!["PCI\\VEN_1234&DEV_5678".into()],
            compatible_ids: vec![],
            present: true,
            problem_code: None,
            problem_status: None,
            condition: DeviceCondition::Missing,
            installed_driver: None,
        };
        let saved = save_scan_to_connection(&mut connection, vec![device.clone()]).unwrap();
        assert_eq!(saved.summary.missing_count, 1);
        assert_eq!(list_scans_from_connection(&connection).unwrap().len(), 1);
        assert_eq!(
            load_scan_from_connection(&connection, saved.summary.id)
                .unwrap()
                .unwrap()
                .devices,
            vec![device]
        );
    }
}
