use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};

use crate::domain::CacheStats;

#[derive(Clone, Debug)]
pub struct MetadataCache {
    path: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CachedValue<T> {
    pub value: T,
    pub fetched_at: i64,
}

impl MetadataCache {
    pub fn open(app_data_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(app_data_dir)
            .map_err(|error| format!("Could not create the DrvMatch data directory: {error}"))?;
        let cache = Self {
            path: app_data_dir.join("metadata.sqlite3"),
        };
        recover_corrupt_database(&cache.path)?;
        initialize(&cache.connection()?)?;
        Ok(cache)
    }

    fn connection(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.path).map_err(database_error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(database_error)?;
        Ok(connection)
    }

    pub fn get<T: DeserializeOwned>(
        &self,
        source: &str,
        cache_key: &str,
    ) -> Result<Option<CachedValue<T>>, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        get_from_connection(&connection, source, cache_key, unix_timestamp())
    }

    pub fn put<T: Serialize>(
        &self,
        source: &str,
        cache_key: &str,
        ttl_seconds: i64,
        value: &T,
    ) -> Result<i64, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        let fetched_at = unix_timestamp();
        put_to_connection(
            &connection,
            source,
            cache_key,
            fetched_at,
            fetched_at.saturating_add(ttl_seconds),
            value,
        )?;
        Ok(fetched_at)
    }

    pub fn stats(&self) -> Result<CacheStats, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        let count = connection
            .query_row("SELECT COUNT(*) FROM source_metadata_cache", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(database_error)?;
        let file_size_bytes = fs::metadata(&self.path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        Ok(CacheStats {
            entry_count: count.max(0) as usize,
            file_size_bytes,
        })
    }

    pub fn clear(&self) -> Result<CacheStats, String> {
        let connection = self.connection()?;
        initialize(&connection)?;
        connection
            .execute_batch("DELETE FROM source_metadata_cache; VACUUM;")
            .map_err(database_error)?;
        drop(connection);
        self.stats()
    }
}

fn initialize(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS source_metadata_cache (
               source TEXT NOT NULL,
               cache_key TEXT NOT NULL,
               fetched_at INTEGER NOT NULL,
               expires_at INTEGER NOT NULL,
               payload_json TEXT NOT NULL,
               PRIMARY KEY (source, cache_key)
             );
             CREATE INDEX IF NOT EXISTS source_metadata_expiry
               ON source_metadata_cache(expires_at);",
        )
        .map_err(database_error)
}

fn recover_corrupt_database(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let valid_header = fs::metadata(path)
        .map(|metadata| metadata.len() == 0)
        .unwrap_or(false)
        || FileHeader::read(path).is_some_and(|header| header == *b"SQLite format 3\0");
    let quick_check_ok = valid_header
        && Connection::open(path)
            .and_then(|connection| {
                connection.query_row("PRAGMA quick_check(1)", [], |row| row.get::<_, String>(0))
            })
            .is_ok_and(|result| result == "ok");
    if quick_check_ok {
        return Ok(());
    }

    let timestamp = unix_timestamp();
    let backup = path.with_extension(format!("sqlite3.corrupt-{timestamp}"));
    fs::rename(path, &backup).map_err(|error| {
        format!(
            "The metadata cache is corrupt and could not be preserved as {}: {error}",
            backup.display()
        )
    })?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", path.display()));
        if sidecar.exists() {
            let sidecar_backup = PathBuf::from(format!("{}{suffix}", backup.display()));
            fs::rename(&sidecar, sidecar_backup).map_err(|error| {
                format!(
                    "Could not preserve corrupt cache sidecar {}: {error}",
                    sidecar.display()
                )
            })?;
        }
    }
    Ok(())
}

struct FileHeader;

impl FileHeader {
    fn read(path: &Path) -> Option<[u8; 16]> {
        let mut file = fs::File::open(path).ok()?;
        let mut header = [0u8; 16];
        file.read_exact(&mut header).ok()?;
        Some(header)
    }
}

fn get_from_connection<T: DeserializeOwned>(
    connection: &Connection,
    source: &str,
    cache_key: &str,
    now: i64,
) -> Result<Option<CachedValue<T>>, String> {
    let row = connection
        .query_row(
            "SELECT fetched_at, payload_json
             FROM source_metadata_cache
             WHERE source = ?1 AND cache_key = ?2 AND expires_at >= ?3",
            params![source, cache_key, now],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(database_error)?;
    let Some((fetched_at, payload)) = row else {
        return Ok(None);
    };
    match serde_json::from_str(&payload) {
        Ok(value) => Ok(Some(CachedValue { value, fetched_at })),
        Err(_) => {
            connection
                .execute(
                    "DELETE FROM source_metadata_cache WHERE source = ?1 AND cache_key = ?2",
                    params![source, cache_key],
                )
                .map_err(database_error)?;
            Ok(None)
        }
    }
}

fn put_to_connection<T: Serialize>(
    connection: &Connection,
    source: &str,
    cache_key: &str,
    fetched_at: i64,
    expires_at: i64,
    value: &T,
) -> Result<(), String> {
    let payload = serde_json::to_string(value)
        .map_err(|error| format!("Could not serialize source metadata: {error}"))?;
    connection
        .execute(
            "INSERT INTO source_metadata_cache(source, cache_key, fetched_at, expires_at, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(source, cache_key) DO UPDATE SET
               fetched_at = excluded.fetched_at,
               expires_at = excluded.expires_at,
               payload_json = excluded.payload_json",
            params![source, cache_key, fetched_at, expires_at, payload],
        )
        .map_err(database_error)?;
    Ok(())
}

pub fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as i64)
}

fn database_error(error: rusqlite::Error) -> String {
    format!("Metadata cache database error: {error}")
}

#[cfg(test)]
mod tests {
    use super::{MetadataCache, get_from_connection, initialize, put_to_connection};
    use rusqlite::Connection;

    #[test]
    fn cache_round_trips_before_expiry() {
        let connection = Connection::open_in_memory().unwrap();
        initialize(&connection).unwrap();
        put_to_connection(
            &connection,
            "catalog",
            "device",
            100,
            200,
            &vec!["candidate"],
        )
        .unwrap();

        let cached = get_from_connection::<Vec<String>>(&connection, "catalog", "device", 150)
            .unwrap()
            .unwrap();
        assert_eq!(cached.fetched_at, 100);
        assert_eq!(cached.value, vec!["candidate"]);
    }

    #[test]
    fn expired_cache_is_not_returned() {
        let connection = Connection::open_in_memory().unwrap();
        initialize(&connection).unwrap();
        put_to_connection(
            &connection,
            "catalog",
            "device",
            100,
            149,
            &vec!["candidate"],
        )
        .unwrap();

        assert!(
            get_from_connection::<Vec<String>>(&connection, "catalog", "device", 150)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn cache_stats_and_clear_reflect_stored_entries() {
        let directory = std::env::temp_dir().join(format!("drvmatch-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let cache = MetadataCache::open(&directory).unwrap();
        cache
            .put("catalog", "device", 60, &vec!["candidate"])
            .unwrap();
        assert_eq!(cache.stats().unwrap().entry_count, 1);
        assert_eq!(cache.clear().unwrap().entry_count, 0);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn malformed_entry_is_evicted_and_treated_as_a_cache_miss() {
        let connection = Connection::open_in_memory().unwrap();
        initialize(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO source_metadata_cache(source, cache_key, fetched_at, expires_at, payload_json) VALUES ('catalog', 'device', 100, 200, '{broken')",
                [],
            )
            .unwrap();

        assert!(
            get_from_connection::<Vec<String>>(&connection, "catalog", "device", 150)
                .unwrap()
                .is_none()
        );
        let remaining: i64 = connection
            .query_row("SELECT COUNT(*) FROM source_metadata_cache", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn corrupt_database_is_preserved_and_recreated() {
        let directory = std::env::temp_dir().join(format!(
            "drvmatch-corrupt-cache-{}-{}",
            std::process::id(),
            super::unix_timestamp()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("metadata.sqlite3"), b"not a sqlite database").unwrap();

        let cache = MetadataCache::open(&directory).unwrap();
        assert_eq!(cache.stats().unwrap().entry_count, 0);
        assert!(
            std::fs::read_dir(&directory)
                .unwrap()
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}
