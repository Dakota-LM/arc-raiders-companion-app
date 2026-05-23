//! Persistent L2 cache layer backed by redb.
//!
//! Stores each cached value in a JSON envelope carrying a `cached_at` timestamp,
//! so freshness can be enforced (redb itself has no expiry). All redb/serde errors
//! are treated as a soft miss: reads return `None`, writes log and continue. The
//! cache is therefore always optional and never breaks the API path.

use crate::services::source::L2State;
use redb::{Database, ReadableDatabase, TableDefinition};
use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Envelope wrapping every stored value with the time it was written (unix seconds).
#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEnvelope<T> {
    cached_at: i64,
    data: T,
}

/// Current unix time in whole seconds (0 if the clock is before the epoch).
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Location of the cache file: the platform data dir, falling back to the temp dir.
fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("arc-companion")
        .join("cache.redb")
}

/// The single shared redb database. `None` if it could not be opened (cache disabled).
static DB: LazyLock<Option<Database>> = LazyLock::new(|| {
    let path = db_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("redb: failed to create cache dir {}: {e}", parent.display());
            return None;
        }
    }
    match Database::create(&path) {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("redb: failed to open cache db at {}: {e}", path.display());
            None
        }
    }
});

/// Force-open the cache database at startup so any failure surfaces in logs early.
/// Safe to call once from `main()`; the cache stays optional either way.
pub fn init() {
    if DB.is_none() {
        eprintln!("redb: persistent cache disabled (database unavailable)");
    }
}

/// Read a fresh (within `ttl`) value from the cache, or `None`.
pub fn read_fresh<T: DeserializeOwned>(
    table: TableDefinition<&str, &[u8]>,
    key: &str,
    ttl: Duration,
) -> Option<T> {
    read_fresh_in(DB.as_ref()?, table, key, ttl)
}

/// Read a value of any age from the cache (offline fallback), or `None`.
pub fn read_stale<T: DeserializeOwned>(
    table: TableDefinition<&str, &[u8]>,
    key: &str,
) -> Option<T> {
    read_stale_in(DB.as_ref()?, table, key)
}

/// Write a value to the cache. No-op if the cache is unavailable; errors are logged.
pub fn write<T: Serialize>(table: TableDefinition<&str, &[u8]>, key: &str, value: &T) {
    if let Some(db) = DB.as_ref() {
        write_in(db, table, key, value);
    }
}

/// Remove a key from the cache. No-op if the cache is unavailable or the key is absent;
/// errors are logged. Used by the `invalidate_*_cache` helpers to also flush the L2 tier.
pub fn remove(table: TableDefinition<&str, &[u8]>, key: &str) {
    if let Some(db) = DB.as_ref() {
        remove_in(db, table, key);
    }
}

/// Read the raw `(cached_at, data)` for a key, or `None` on any miss/error (soft miss).
fn read_raw_in<T: DeserializeOwned>(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
) -> Option<(i64, T)> {
    let txn = db.begin_read().ok()?;
    let tbl = txn.open_table(table).ok()?;
    let guard = tbl.get(key).ok()??;
    let env: CacheEnvelope<T> = serde_json::from_slice(guard.value()).ok()?;
    Some((env.cached_at, env.data))
}

/// Return the value only if it exists and is within `ttl`; otherwise `None`.
fn read_fresh_in<T: DeserializeOwned>(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
    ttl: Duration,
) -> Option<T> {
    let (cached_at, data) = read_raw_in(db, table, key)?;
    // A negative age means a future-dated entry (clock skew) — short-circuit to stale
    // so the `as u64` cast never wraps a negative into a huge value.
    let age = now_secs().saturating_sub(cached_at);
    if age >= 0 && (age as u64) <= ttl.as_secs() {
        Some(data)
    } else {
        None
    }
}

/// Return the value regardless of age (offline fallback); `None` only if absent/unreadable.
fn read_stale_in<T: DeserializeOwned>(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
) -> Option<T> {
    read_raw_in(db, table, key).map(|(_, data)| data)
}

/// Serialize `value` into a timestamped envelope and write it. Errors are logged, not propagated.
fn write_in<T: Serialize>(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
    value: &T,
) {
    let env = CacheEnvelope { cached_at: now_secs(), data: value };
    let bytes = match serde_json::to_vec(&env) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("redb: serialize failed for key {key}: {e}");
            return;
        }
    };
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let txn = db.begin_write()?;
        {
            let mut tbl = txn.open_table(table)?;
            tbl.insert(key, bytes.as_slice())?;
        }
        txn.commit()?;
        Ok(())
    })();
    if let Err(e) = result {
        eprintln!("redb: write failed for key {key}: {e}");
    }
}

/// Minimal view of an envelope used by `l2_state` to read just the timestamp,
/// ignoring the (possibly large) `data` field (serde skips unknown fields).
#[derive(serde::Deserialize)]
struct CachedAtProbe {
    cached_at: i64,
}

/// Classify the redb (L2) state for `key` without deserializing the payload.
fn l2_state_in(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
    ttl: Duration,
) -> L2State {
    let cached_at = (|| -> Option<i64> {
        let txn = db.begin_read().ok()?;
        let tbl = txn.open_table(table).ok()?;
        let guard = tbl.get(key).ok()??;
        let probe: CachedAtProbe = serde_json::from_slice(guard.value()).ok()?;
        Some(probe.cached_at)
    })();

    match cached_at {
        Some(ts) => {
            let age = now_secs().saturating_sub(ts);
            if age >= 0 && (age as u64) <= ttl.as_secs() {
                L2State::Fresh
            } else {
                L2State::Stale
            }
        }
        None => L2State::Miss,
    }
}

/// Dev-diagnostic probe of the redb (L2) state for `key`. Returns `Miss` if the
/// cache is unavailable. Read-only; does not touch Moka.
pub fn l2_state(table: TableDefinition<&str, &[u8]>, key: &str, ttl: Duration) -> L2State {
    match DB.as_ref() {
        Some(db) => l2_state_in(db, table, key, ttl),
        None => L2State::Miss,
    }
}

/// Remove a key from `table`. Missing key/table is a no-op; errors are logged, not propagated.
fn remove_in(db: &Database, table: TableDefinition<&str, &[u8]>, key: &str) {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let txn = db.begin_write()?;
        {
            let mut tbl = txn.open_table(table)?;
            tbl.remove(key)?;
        }
        txn.commit()?;
        Ok(())
    })();
    if let Err(e) = result {
        eprintln!("redb: remove failed for key {key}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TT: TableDefinition<&str, &[u8]> = TableDefinition::new("t");

    /// Fresh temp database. The returned TempDir must be kept alive for the db to stay valid.
    fn temp_db() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::create(dir.path().join("t.redb")).unwrap();
        (dir, db)
    }

    #[test]
    fn write_then_read_fresh_roundtrips() {
        let (_dir, db) = temp_db();
        let data = vec!["a".to_string(), "b".to_string()];
        write_in(&db, TT, "k", &data);
        let got: Option<Vec<String>> = read_fresh_in(&db, TT, "k", Duration::from_secs(900));
        assert_eq!(got, Some(data));
    }

    #[test]
    fn read_fresh_returns_none_when_stale_but_read_stale_returns_it() {
        let (_dir, db) = temp_db();
        // Manually insert an envelope timestamped 10000s in the past.
        let env = CacheEnvelope { cached_at: now_secs() - 10_000, data: vec![1u8, 2, 3] };
        let bytes = serde_json::to_vec(&env).unwrap();
        let txn = db.begin_write().unwrap();
        {
            let mut t = txn.open_table(TT).unwrap();
            t.insert("k", bytes.as_slice()).unwrap();
        }
        txn.commit().unwrap();

        let fresh: Option<Vec<u8>> = read_fresh_in(&db, TT, "k", Duration::from_secs(900));
        assert_eq!(fresh, None, "a 10000s-old entry must be stale under a 900s TTL");

        let stale: Option<Vec<u8>> = read_stale_in(&db, TT, "k");
        assert_eq!(stale, Some(vec![1, 2, 3]), "stale read must still return the value");
    }

    #[test]
    fn db_path_points_at_app_cache_file() {
        let p = db_path();
        assert!(
            p.ends_with("arc-companion/cache.redb"),
            "unexpected cache path: {}",
            p.display()
        );
    }

    #[test]
    fn reads_return_none_for_missing_key_or_table() {
        let (_dir, db) = temp_db();
        let fresh: Option<Vec<u8>> = read_fresh_in(&db, TT, "absent", Duration::from_secs(900));
        let stale: Option<Vec<u8>> = read_stale_in(&db, TT, "absent");
        assert_eq!(fresh, None);
        assert_eq!(stale, None);
    }

    #[test]
    fn remove_deletes_a_written_key() {
        let (_dir, db) = temp_db();
        write_in(&db, TT, "k", &vec![1u8, 2, 3]);
        assert!(read_stale_in::<Vec<u8>>(&db, TT, "k").is_some());
        remove_in(&db, TT, "k");
        assert_eq!(read_stale_in::<Vec<u8>>(&db, TT, "k"), None);
    }

    #[test]
    fn remove_missing_key_is_a_noop() {
        let (_dir, db) = temp_db();
        // Removing an absent key (and creating the table en route) must not panic.
        remove_in(&db, TT, "absent");
        assert_eq!(read_stale_in::<Vec<u8>>(&db, TT, "absent"), None);
    }

    #[test]
    fn l2_state_reports_fresh_stale_and_miss() {
        let (_dir, db) = temp_db();

        // Fresh: written now, within TTL.
        write_in(&db, TT, "fresh", &vec![1u8, 2, 3]);
        assert_eq!(l2_state_in(&db, TT, "fresh", Duration::from_secs(900)), L2State::Fresh);

        // Stale: craft an envelope timestamped 10000s in the past.
        let env = CacheEnvelope { cached_at: now_secs() - 10_000, data: vec![9u8] };
        let bytes = serde_json::to_vec(&env).unwrap();
        let txn = db.begin_write().unwrap();
        {
            let mut t = txn.open_table(TT).unwrap();
            t.insert("stale", bytes.as_slice()).unwrap();
        }
        txn.commit().unwrap();
        assert_eq!(l2_state_in(&db, TT, "stale", Duration::from_secs(900)), L2State::Stale);

        // Miss: absent key.
        assert_eq!(l2_state_in(&db, TT, "absent", Duration::from_secs(900)), L2State::Miss);
    }
}
