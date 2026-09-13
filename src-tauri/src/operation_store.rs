use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{DriverSourceKind, InstallRecord, SourceHealth};

#[derive(Clone, Debug)]
pub struct OperationStore {
    path: PathBuf,
}

impl OperationStore {
    pub fn open(app_data_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(app_data_dir)
            .map_err(|error| format!("Could not create the DrvMatch data directory: {error}"))?;
        let store = Self {
            path: app_data_dir.join("operations.sqlite3"),
        };
        initialize(&store.connection()?)?;
        Ok(store)
    }

    fn connection(&self) -> Result<Connection, String> {
        Connection::open(&self.path).map_err(database_error)
    }

    pub fn save(&self, record: &InstallRecord) -> Result<(), String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        let payload = serde_json::to_string(record)
            .map_err(|error| format!("Could not serialize the installation record: {error}"))?;
        connection.execute(
            "INSERT INTO install_operations(id, started_at, payload_json) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET started_at = excluded.started_at, payload_json = excluded.payload_json",
            params![record.id, record.started_at, payload],
        ).map_err(database_error)?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<InstallRecord>, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        let mut statement = connection
            .prepare(
                "SELECT payload_json FROM install_operations ORDER BY started_at DESC LIMIT 200",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        rows.map(|payload| {
            serde_json::from_str(&payload.map_err(database_error)?)
                .map_err(|error| format!("Stored installation history is invalid: {error}"))
        })
        .collect()
    }

    pub fn load(&self, id: &str) -> Result<Option<InstallRecord>, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        let payload = connection
            .query_row(
                "SELECT payload_json FROM install_operations WHERE id = ?1",
                [id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?;
        payload
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|error| format!("Stored installation history is invalid: {error}"))
            })
            .transpose()
    }

    pub fn save_source_health(&self, sources: &[SourceHealth]) -> Result<(), String> {
        let mut connection = self.connection()?;
        initialize(&connection)?;
        let transaction = connection.transaction().map_err(database_error)?;
        for source in sources {
            let payload = serde_json::to_string(source)
                .map_err(|error| format!("Could not serialize source health: {error}"))?;
            transaction.execute(
                "INSERT INTO source_health(source, checked_at, payload_json) VALUES (?1, ?2, ?3)
                 ON CONFLICT(source) DO UPDATE SET checked_at = excluded.checked_at, payload_json = excluded.payload_json",
                params![source_key(source.source), source.checked_at, payload],
            ).map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)
    }

    pub fn list_source_health(&self) -> Result<Vec<SourceHealth>, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        let mut statement = connection
            .prepare("SELECT payload_json FROM source_health ORDER BY source")
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        rows.map(|payload| {
            serde_json::from_str(&payload.map_err(database_error)?)
                .map_err(|error| format!("Stored source health is invalid: {error}"))
        })
        .collect()
    }
}

fn source_key(source: DriverSourceKind) -> &'static str {
    match source {
        DriverSourceKind::WindowsUpdate => "windows-update",
        DriverSourceKind::MicrosoftCatalog => "microsoft-catalog",
        DriverSourceKind::Amd => "amd",
        DriverSourceKind::Nvidia => "nvidia",
        DriverSourceKind::Intel => "intel",
        DriverSourceKind::Dell => "dell",
        DriverSourceKind::Lenovo => "lenovo",
        DriverSourceKind::Hp => "hp",
    }
}

fn initialize(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
         CREATE TABLE IF NOT EXISTS install_operations (
           id TEXT PRIMARY KEY,
           started_at INTEGER NOT NULL,
           payload_json TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS source_health (
           source TEXT PRIMARY KEY,
           checked_at INTEGER NOT NULL,
           payload_json TEXT NOT NULL
         );",
        )
        .map_err(database_error)
}

fn database_error(error: rusqlite::Error) -> String {
    format!("Installation history database error: {error}")
}

#[cfg(test)]
mod tests {
    use super::OperationStore;
    use crate::domain::{
        DriverSourceKind, InstallRecord, InstallResultState, SourceHealth, SourceHealthState,
    };

    #[test]
    fn install_record_round_trips() {
        let directory =
            std::env::temp_dir().join(format!("drvmatch-operation-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let store = OperationStore::open(&directory).unwrap();
        let record = InstallRecord {
            id: "item-1".into(),
            operation_id: "operation-1".into(),
            started_at: 1,
            completed_at: 2,
            device_instance_id: "PCI\\VEN_1234".into(),
            device_name: "Fixture".into(),
            candidate_id: "candidate".into(),
            candidate_name: "Fixture driver".into(),
            source: DriverSourceKind::MicrosoftCatalog,
            previous_version: Some("1".into()),
            installed_version: Some("2".into()),
            previous_inf: Some("oem1.inf".into()),
            package_sha256: Some("abc".into()),
            signature_verified: true,
            restore_point_attempted: true,
            restore_point_created: false,
            backup_path: None,
            state: InstallResultState::Succeeded,
            message: "Installed".into(),
            reboot_required: false,
            rollback_available: false,
        };
        store.save(&record).unwrap();
        assert_eq!(store.load("item-1").unwrap(), Some(record));
        assert_eq!(store.list().unwrap().len(), 1);
        let health = SourceHealth {
            source: DriverSourceKind::MicrosoftCatalog,
            state: SourceHealthState::Available,
            checked_at: 3,
            cached: true,
            candidate_count: 2,
            message: None,
        };
        store
            .save_source_health(std::slice::from_ref(&health))
            .unwrap();
        assert_eq!(store.list_source_health().unwrap(), vec![health]);
        let _ = std::fs::remove_dir_all(directory);
    }
}
