//! Persistent L2 cache layer backed by redb.
//!
//! Stores each cached value in a JSON envelope carrying a `cached_at` timestamp,
//! so freshness can be enforced (redb itself has no expiry). All redb/serde errors
//! are treated as a soft miss: reads return `None`, writes log and continue. The
//! cache is therefore always optional and never breaks the API path.

use redb::{Database, ReadableDatabase, TableDefinition};
use serde::{de::DeserializeOwned, Serialize};
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
    fn reads_return_none_for_missing_key_or_table() {
        let (_dir, db) = temp_db();
        let fresh: Option<Vec<u8>> = read_fresh_in(&db, TT, "absent", Duration::from_secs(900));
        let stale: Option<Vec<u8>> = read_stale_in(&db, TT, "absent");
        assert_eq!(fresh, None);
        assert_eq!(stale, None);
    }
}
