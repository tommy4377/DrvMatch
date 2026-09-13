use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{AppSettings, DriverSourceKind};

const MAX_LOG_READ_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn open(app_data_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(app_data_dir)
            .map_err(|error| format!("Could not create the DrvMatch data directory: {error}"))?;
        let store = Self {
            path: app_data_dir.join("settings.sqlite3"),
        };
        initialize(&store.connection()?)?;
        if store.load()?.is_none() {
            store.save(&AppSettings::default())?;
        }
        Ok(store)
    }

    fn connection(&self) -> Result<Connection, String> {
        Connection::open(&self.path).map_err(database_error)
    }

    pub fn get(&self) -> Result<AppSettings, String> {
        self.load()?
            .ok_or_else(|| "Application settings were not initialized.".into())
    }

    fn load(&self) -> Result<Option<AppSettings>, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        let payload = connection
            .query_row(
                "SELECT payload_json FROM app_settings WHERE id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?;
        payload
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|error| format!("Stored application settings are invalid: {error}"))
            })
            .transpose()
    }

    pub fn save(&self, settings: &AppSettings) -> Result<AppSettings, String> {
        validate(settings)?;
        let payload = serde_json::to_string(settings)
            .map_err(|error| format!("Could not serialize application settings: {error}"))?;
        let connection = self.connection()?;
        initialize(&connection)?;
        connection.execute(
            "INSERT INTO app_settings(id, payload_json, updated_at) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET payload_json = excluded.payload_json, updated_at = excluded.updated_at",
            params![payload, now_seconds()],
        ).map_err(database_error)?;
        Ok(settings.clone())
    }
}

fn initialize(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
         CREATE TABLE IF NOT EXISTS app_settings (
           id INTEGER PRIMARY KEY CHECK(id = 1),
           payload_json TEXT NOT NULL,
           updated_at INTEGER NOT NULL
         );",
        )
        .map_err(database_error)
}

fn validate(settings: &AppSettings) -> Result<(), String> {
    if settings.enabled_sources.is_empty() && settings.enabled_oem_sources.is_empty() {
        return Err("At least one trusted driver source must remain enabled.".into());
    }
    if settings.enabled_sources.iter().any(|source| {
        matches!(
            source,
            DriverSourceKind::Dell | DriverSourceKind::Lenovo | DriverSourceKind::Hp
        )
    }) {
        return Err("System OEM sources must be stored in the OEM source group.".into());
    }
    if settings.enabled_oem_sources.iter().any(|source| {
        !matches!(
            source,
            DriverSourceKind::Dell | DriverSourceKind::Lenovo | DriverSourceKind::Hp
        )
    }) {
        return Err(
            "Only supported system OEM sources may be enabled in the OEM source group.".into(),
        );
    }
    let mut unique = settings.enabled_sources.clone();
    unique.extend(&settings.enabled_oem_sources);
    unique.sort_by_key(|source| *source as u8);
    unique.dedup();
    if unique.len() != settings.enabled_sources.len() + settings.enabled_oem_sources.len() {
        return Err("Driver sources cannot contain duplicate entries.".into());
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ActivityLog {
    path: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

impl ActivityLog {
    pub fn open(app_data_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(app_data_dir)
            .map_err(|error| format!("Could not create the DrvMatch data directory: {error}"))?;
        Ok(Self {
            path: app_data_dir.join("drvmatch.log"),
            write_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn write(&self, level: &str, area: &str, message: &str) {
        let Ok(_guard) = self.write_lock.lock() else {
            return;
        };
        let clean = message.replace(['\r', '\n'], " ");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "{} [{level}] {area}: {clean}", now_seconds());
        }
    }

    pub fn read_tail(&self) -> Result<String, String> {
        let Ok(_guard) = self.write_lock.lock() else {
            return Err("The activity log is busy.".into());
        };
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
            Err(error) => return Err(format!("Could not open the activity log: {error}")),
        };
        let length = file
            .metadata()
            .map_err(|error| format!("Could not inspect the activity log: {error}"))?
            .len();
        file.seek(SeekFrom::Start(length.saturating_sub(MAX_LOG_READ_BYTES)))
            .map_err(|error| format!("Could not seek the activity log: {error}"))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| format!("Could not read the activity log: {error}"))?;
        let mut output = String::from_utf8_lossy(&bytes).into_owned();
        if length > MAX_LOG_READ_BYTES {
            if let Some(first_line) = output.find('\n') {
                output.drain(..=first_line);
            }
            output.insert_str(0, "[Earlier log entries omitted]\n");
        }
        Ok(output)
    }

    pub fn clear(&self) -> Result<(), String> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| "The activity log is busy.".to_string())?;
        File::create(&self.path)
            .map_err(|error| format!("Could not clear the activity log: {error}"))?;
        Ok(())
    }
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_secs() as i64)
}

fn database_error(error: rusqlite::Error) -> String {
    format!("Settings database error: {error}")
}

#[cfg(test)]
mod tests {
    use super::{ActivityLog, SettingsStore};
    use crate::domain::DriverSourceKind;

    #[test]
    fn settings_persist_and_require_a_source() {
        let directory =
            std::env::temp_dir().join(format!("drvmatch-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let store = SettingsStore::open(&directory).unwrap();
        let mut settings = store.get().unwrap();
        settings.acrylic = false;
        settings.enabled_sources = vec![DriverSourceKind::WindowsUpdate];
        store.save(&settings).unwrap();
        assert_eq!(
            SettingsStore::open(&directory).unwrap().get().unwrap(),
            settings
        );
        settings.enabled_sources.clear();
        settings.enabled_oem_sources.clear();
        assert!(store.save(&settings).is_err());

        settings.enabled_sources = vec![DriverSourceKind::Dell];
        assert!(store.save(&settings).is_err());

        settings.enabled_sources = vec![DriverSourceKind::WindowsUpdate];
        settings.enabled_oem_sources = vec![DriverSourceKind::Intel];
        assert!(store.save(&settings).is_err());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn activity_log_is_readable_and_clearable() {
        let directory = std::env::temp_dir().join(format!("drvmatch-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let log = ActivityLog::open(&directory).unwrap();
        log.write("INFO", "test", "one\nline");
        assert!(log.read_tail().unwrap().contains("test: one line"));
        log.clear().unwrap();
        assert!(log.read_tail().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(directory);
    }
}
