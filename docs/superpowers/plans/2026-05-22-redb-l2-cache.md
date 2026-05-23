# redb L2 Persistent Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add redb as a persistent L2 cache between Moka (L1) and the MetaForge API, wired end-to-end through the `items` service as the pilot.

**Architecture:** A new shared `services/db.rs` module owns a single redb `Database` handle (a `LazyLock<Option<Database>>`, mirroring the existing `HTTP_CLIENT` pattern) and provides generic `read_fresh` / `read_stale` / `write` helpers over a JSON envelope carrying a `cached_at` timestamp. Each data service keeps its own `get_*()` but switches to Moka's `entry().or_try_insert_with()` loader, whose closure runs only on an L1 miss and drives the cascade: **fresh redb → API (write-through) → stale redb (offline fallback)**. redb is always treated as optional — any redb/serde error is a soft miss that never breaks the API path.

**Tech Stack:** Rust, Dioxus 0.7, `moka::sync` 0.12, `redb` 4.1, `serde` + `serde_json`, `dirs` 6. Models from `arc_api_rs` 0.2 (already derive `Serialize`/`Deserialize`).

**Reference spec:** `docs/superpowers/specs/2026-05-22-redb-l2-cache-design.md`

**Branch:** Execute on a feature branch (e.g. `feat/redb-l2-cache`), not `main`.

**Canonical commands** (the default `mobile` feature does not link on a desktop host, so always pass desktop):
- Test: `cargo test --no-default-features --features desktop`
- Check: `cargo check --no-default-features --features desktop`

---

## File Structure

- **Create** `src/services/db.rs` — the only shared module. Owns the redb handle, the JSON envelope, serialization, the TTL/freshness check, and the public `read_fresh`/`read_stale`/`write`/`init` API. Internal `*_in(db, …)` functions take a `&Database` so they're unit-testable against a temp database.
- **Modify** `src/services/mod.rs` — register `pub mod db;`.
- **Modify** `src/main.rs` — call `services::db::init()` before `dioxus::launch`.
- **Modify** `src/services/items.rs` — add `ITEMS_TABLE`, convert `fetch_items_isolated` (async) to a sync `fetch_items_blocking`, and rewrite `get_all_items` to the Moka-loader cascade. `ItemsResult`, `DataSource`, the Moka cache, and `invalidate_items_cache` are unchanged. No view files change.
- **Modify** `Cargo.toml` — add `serde`, `serde_json`, `dirs` deps; add `tempfile` dev-dep.

`events.rs`, `bots.rs`, `traders.rs` and all views are out of scope for this pilot (fast-follow).

---

### Task 1: Add dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the runtime and dev dependencies**

In `Cargo.toml`, under `[dependencies]` (after the existing `redb = "4"` line) add:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "6"
```

Then add a new section after the `[dependencies]` block (before `[features]`):

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Verify it resolves and still compiles**

Run: `cargo check --no-default-features --features desktop`
Expected: finishes with `Finished` (warnings OK), no dependency-resolution or compile errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build(cache): add serde, serde_json, dirs, tempfile deps for redb L2"
```

---

### Task 2: redb helper core (TDD)

Pure, testable functions over a passed-in `&Database`. The static handle and public wrappers come in Task 3.

**Files:**
- Create: `src/services/db.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/services/db.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/services/db.rs` with the envelope, the timestamp helper, and the test module. The functions under test (`write_in`, `read_fresh_in`, `read_stale_in`) do not exist yet, so this will not compile — that is the expected RED.

```rust
//! Persistent L2 cache layer backed by redb.
//!
//! Stores each cached value in a JSON envelope carrying a `cached_at` timestamp,
//! so freshness can be enforced (redb itself has no expiry). All redb/serde errors
//! are treated as a soft miss: reads return `None`, writes log and continue. The
//! cache is therefore always optional and never breaks the API path.

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --no-default-features --features desktop services::db`
Expected: FAIL — compile error, `cannot find function write_in` / `read_fresh_in` / `read_stale_in` in this scope.

- [ ] **Step 3: Implement the helpers**

Add these functions to `src/services/db.rs`, immediately after `now_secs` (before the `tests` module):

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --no-default-features --features desktop services::db`
Expected: PASS — 3 tests in `services::db::tests` pass.

- [ ] **Step 5: Commit**

```bash
git add src/services/db.rs
git commit -m "feat(cache): add redb envelope read/write helpers with TTL freshness"
```

---

### Task 3: redb handle + public API + startup init

Wire the global handle, the public wrappers, the module registration, and the startup call.

**Files:**
- Modify: `src/services/db.rs`
- Modify: `src/services/mod.rs`
- Modify: `src/main.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/services/db.rs`

- [ ] **Step 1: Write the failing test for the db path**

In `src/services/db.rs`, inside the existing `mod tests`, add this test (references `db_path`, which does not exist yet → RED):

```rust
    #[test]
    fn db_path_points_at_app_cache_file() {
        let p = db_path();
        assert!(
            p.ends_with("arc-companion/cache.redb"),
            "unexpected cache path: {}",
            p.display()
        );
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --no-default-features --features desktop services::db`
Expected: FAIL — compile error, `cannot find function db_path in this scope`.

- [ ] **Step 3: Add the handle, path, init, and public wrappers**

In `src/services/db.rs`, extend the top `use` block to add:

```rust
use std::path::PathBuf;
use std::sync::LazyLock;
```

Then add the following after the `now_secs` function (before `read_raw_in`):

```rust
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
```

- [ ] **Step 4: Register the module**

In `src/services/mod.rs`, add this line in the `pub mod` list (e.g. directly after `pub mod httpclientbuilder;`):

```rust
pub mod db;
```

- [ ] **Step 5: Initialize at startup**

In `src/main.rs`, change `main` from:

```rust
fn main() {
    dioxus::launch(App);
}
```

to:

```rust
fn main() {
    services::db::init();
    dioxus::launch(App);
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --no-default-features --features desktop services::db`
Expected: PASS — all 4 tests in `services::db::tests` pass.

- [ ] **Step 7: Commit**

```bash
git add src/services/db.rs src/services/mod.rs src/main.rs
git commit -m "feat(cache): add shared redb handle, public read/write API, startup init"
```

---

### Task 4: Wire redb into the items service

Switch `get_all_items` to the Moka-loader cascade using the `db` helpers. This path involves the network and Dioxus, so it is verified by build + existing/db tests + a manual run rather than a new unit test.

**Files:**
- Modify: `src/services/items.rs`

- [ ] **Step 1: Add imports and the table definition**

In `src/services/items.rs`, add to the `use` block:

```rust
use crate::services::db;
use redb::TableDefinition;
use std::cell::RefCell;
```

Add this constant after the existing `const ITEMS_CACHE_KEY: &str = "all_items";` line:

```rust
const ITEMS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("items");
```

- [ ] **Step 2: Convert the fetcher from async to a blocking fn**

The current `fetch_items_isolated` is declared `async` but contains no `.await` — it spawns a thread and blocks on `rx.recv()`. The Moka loader closure is synchronous, so make it a plain `fn`. Change the signature from:

```rust
async fn fetch_items_isolated() -> Result<Vec<Item>, String> {
```

to:

```rust
fn fetch_items_blocking() -> Result<Vec<Item>, String> {
```

Leave the entire body unchanged.

- [ ] **Step 3: Rewrite `get_all_items` to the loader cascade**

Replace the whole `get_all_items` function with:

```rust
pub async fn get_all_items() -> ItemsResult {
    // Captures which branch the loader took, so we can report the right DataSource.
    let resolved: RefCell<Option<DataSource>> = RefCell::new(None);

    // Moka L1 loader: the closure runs only on an L1 miss, and on success its value
    // is inserted into Moka (warming the cache). On Err nothing is cached.
    let outcome = ITEMS_CACHE
        .entry(ITEMS_CACHE_KEY.to_string())
        .or_try_insert_with(|| -> Result<Vec<Item>, String> {
            // L2: fresh redb
            if let Some(items) = db::read_fresh::<Vec<Item>>(
                ITEMS_TABLE,
                ITEMS_CACHE_KEY,
                Duration::from_secs(CACHE_TTL_SECS),
            ) {
                *resolved.borrow_mut() = Some(DataSource::Cache);
                return Ok(items);
            }
            // Source: API (write-through to redb on success)
            match fetch_items_blocking() {
                Ok(items) => {
                    db::write(ITEMS_TABLE, ITEMS_CACHE_KEY, &items);
                    *resolved.borrow_mut() = Some(DataSource::Api);
                    Ok(items)
                }
                // Offline fallback: serve stale redb if present, else propagate the error.
                Err(err) => match db::read_stale::<Vec<Item>>(ITEMS_TABLE, ITEMS_CACHE_KEY) {
                    Some(items) => {
                        *resolved.borrow_mut() = Some(DataSource::Cache);
                        Ok(items)
                    }
                    None => Err(err),
                },
            }
        });

    match outcome {
        Ok(entry) => {
            // is_fresh() == false means the value was already in Moka (an L1 hit).
            let source = if entry.is_fresh() {
                resolved.borrow().clone().unwrap_or(DataSource::Api)
            } else {
                DataSource::Cache
            };
            let items = entry.into_value();
            let count = items.len();
            ItemsResult {
                items,
                source,
                count,
                error: None,
            }
        }
        Err(err) => ItemsResult {
            items: Vec::new(),
            source: DataSource::Api,
            count: 0,
            error: Some((*err).clone()),
        },
    }
}
```

- [ ] **Step 4: Verify it compiles and all tests pass**

Run: `cargo test --no-default-features --features desktop`
Expected: PASS — the crate compiles; `services::db::tests` and all pre-existing component tests pass.

- [ ] **Step 5: Lint check**

Run: `cargo clippy --no-default-features --features desktop`
Expected: no new errors. (The `await-holding-invalid-types` lint in `clippy.toml` targets dioxus/generational-box types; the new code holds only a `RefCell` borrow and across no `.await`, so it is unaffected.)

- [ ] **Step 6: Manual acceptance run**

Launch the app (desktop is the host-friendly target):

Run: `dx serve --platform desktop`  (fallback: `cargo run --no-default-features --features desktop`)

Verify:
1. **Cold start:** open the Materials/items page — the `Spinner` shows briefly, then items render (fetched from the API).
2. **redb file created:** confirm the cache file exists. On Linux: `ls -l ~/.local/share/arc-companion/cache.redb` (the path is `dirs::data_dir()/arc-companion/cache.redb`).
3. **Warm restart:** stop and relaunch the app, open the items page again — items appear with little or no spinner (served from redb on the first Moka miss, then warmed into Moka).

- [ ] **Step 7: Commit**

```bash
git add src/services/items.rs
git commit -m "feat(items): use redb L2 cache via Moka loader cascade"
```

---

## Self-Review Notes

**Spec coverage:**
- Cascade Moka→fresh redb→API→stale redb → Task 4 `get_all_items`. ✓
- TTL-gated freshness + offline fallback → Task 2 (`read_fresh_in` TTL check, `read_stale_in`) + Task 4 (stale branch). ✓
- `services/db.rs` shared helper, `LazyLock` handle mirroring `HTTP_CLIENT`, `init()` from `main` → Tasks 2–3. ✓
- One table per domain; `items` table only in pilot → Task 4 `ITEMS_TABLE`. ✓
- serde_json envelope with `cached_at` → Task 2 `CacheEnvelope`. ✓
- `dirs` data-dir path → Task 3 `db_path`. ✓
- Loading UI unchanged → no view edits; spinner behavior preserved (Task 4 manual check). ✓
- redb errors as soft miss (never breaks API path) → `Option<Database>` + `.ok()?` + logged writes. ✓

**Type consistency:** `read_fresh`/`read_stale`/`write` signatures identical across Tasks 2–4; `ITEMS_TABLE: TableDefinition<&str, &[u8]>` matches helper params; `or_try_insert_with` error type `String` matches `ItemsResult.error: Option<String>` via `(*err).clone()`.

**Out of scope (fast-follow):** events, bots, traders services; web/WASM target; mobile cache-path validation (desktop path verified in Task 4 step 6).

**Known minor limitation:** under a rare concurrent first-load, a waiter thread served the value computed by another thread reports `source = Api` by default (its loader closure didn't run to set `resolved`). This affects only the debug `source` label, not correctness.
