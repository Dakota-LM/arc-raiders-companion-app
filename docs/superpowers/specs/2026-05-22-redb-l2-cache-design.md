# redb as L2 Persistent Cache — Design

**Date:** 2026-05-22
**Status:** Approved (design)
**Scope of first pass:** Pilot through the `items` service only.

## Problem

The app's data services (`events`, `items`, `bots`, `traders`) currently use a single
in-memory cache tier: a `moka::sync::Cache` per service (TTL 900s). On any cache miss —
including every cold start, since Moka is in-memory and dies with the process — the service
hits the MetaForge API. This means:

- Every app launch re-fetches everything from the network.
- If the API is unreachable at launch, the user sees nothing (no persisted fallback).

We already depend on `redb 4` but don't use it. The goal is to introduce redb as a
persistent **L2** cache between Moka (**L1**) and the API (**source**), so data survives
restarts and provides an offline fallback.

## Cascade

Moka's loader (`try_get_with`) orchestrates the cascade. This replaces today's manual
`cache.get()` / `cache.insert()` in each service. The loader closure runs **only on an L1
miss** and dedupes concurrent callers (two pages requesting the same data won't double-fetch).

Order of resolution inside the loader closure:

1. **Fresh L2** — `db::read_fresh(TABLE, key, TTL)`. If a redb entry exists and its
   `cached_at` is within TTL, return it. Returning warms Moka. `source = Cache`.
2. **Source / API** — otherwise call the existing `fetch_*_isolated()`. On success,
   `db::write(TABLE, key, &value)` (write-through to redb), then return. `source = Api`.
3. **Stale L2 (offline fallback)** — if the API call errors, `db::read_stale(TABLE, key)`.
   If an expired redb copy exists, serve it. `source = Cache` (or `Fallback` for traders).
   Only if that is also empty do we surface the error.

```text
get_*()  ──►  moka.try_get_with(key, closure)
                         │ (L1 miss only)
                         ▼
              db::read_fresh(table, key, ttl)? ──► return (warm Moka)   [Cache]
                         │ none/stale
                         ▼
              fetch_*_isolated()
                 ├─ Ok(v)  ──► db::write(table, key, &v); return         [Api]
                 └─ Err(e) ──► db::read_stale(table, key)?
                                    ├─ Some(v) ──► return                [Cache/Fallback]
                                    └─ None    ──► surface error
```

### Freshness policy

redb has no built-in expiry, so freshness is enforced by us:

- Each stored value is wrapped in a JSON envelope carrying a `cached_at` timestamp.
- **Online & fresh:** redb entry within TTL is used directly.
- **Online & stale/missing:** API is hit and the entry is refreshed (write-through).
- **Offline:** a stale redb entry is served as a last resort so the user still sees data.

The TTL constant reuses each service's existing `CACHE_TTL_SECS` (900s) for parity with L1.

## Components

### `services/db.rs` (new, shared)

The **only** thing factored out of the services. It owns the redb handle and the byte-level
work (open table, serialize/deserialize, timestamp check) so that plumbing isn't copy-pasted
into four services. It does **not** own the cascade flow — that stays in each service's
`get_*()`.

- `static DB: LazyLock<Database>` — opens one redb file, mirroring the existing
  `HTTP_CLIENT: LazyLock<Client>` pattern in `services/httpclientbuilder.rs`.
- `pub fn init()` — `LazyLock::force(&DB)`; called once from `main()` so the file is created
  and any open error surfaces at startup rather than on first data access.
- Internal envelope: `struct CacheEnvelope<T> { cached_at: i64, data: T }`, serialized with
  `serde_json` to bytes.
- Public helpers (generic over the value type):
  - `read_fresh<T: DeserializeOwned>(table, key, ttl: Duration) -> Option<T>`
  - `read_stale<T: DeserializeOwned>(table, key) -> Option<T>`
  - `write<T: Serialize>(table, key, value: &T)`
- Tables are typed `TableDefinition<&str, &[u8]>`. Exact redb 4 transaction API
  (`begin_write` / `open_table` / `insert` / `commit`, `begin_read` / `get`) to be confirmed
  against the redb 4 docs during planning.
- Errors (corrupt file, deserialize failure, write failure) are treated as a **soft miss** —
  the helper returns `None` (reads) or logs and continues (writes). redb being unavailable
  must never break the API path.

### Per-service changes (pilot: `items.rs`)

- Add a `const ITEMS_TABLE: TableDefinition<&str, &[u8]>`.
- Rewrite the body of `get_all_items()` to use `ITEMS_CACHE.try_get_with(...)` with the
  cascade closure above. The closure also determines the `source` for `ItemsResult`.
- **Unchanged:** `fetch_items_isolated()`, the `ItemsResult` struct, the `DataSource` enum,
  the Moka cache definition, and all view-facing types. Views (`items_view.rs`) are untouched.

### Tables

One table per domain, all in the single redb file:

| Table          | Key                              | Value (envelope `data`)   |
|----------------|----------------------------------|---------------------------|
| `items`        | `"all_items"`                    | `Vec<Item>`               |
| `events`       | `"events_schedule"`              | `EventsScheduleResponse`  |
| `bots`         | `"all_bots"`                     | `Vec<Bot>`                |
| `trader_names` | `"trader_names"`                 | `Vec<String>`             |
| `trader_items` | `"trader_items_<name>"`          | trader inventory          |

Only `items` is wired in the pilot; the rest are listed for the fast-follow.

### DB file location

A platform-appropriate, writable data directory — proposed `dirs::data_dir()` joined with an
app folder, e.g. `…/arc-companion/cache.redb`, created if absent. This adds the small `dirs`
crate. The exact strategy for the Android/iOS sandbox path will be confirmed during planning;
desktop is straightforward via `dirs::data_dir()`.

### Initialization

`main()` calls `services::db::init()` before `dioxus::launch(App)` so the redb file is opened
(and any failure logged) at startup.

### Loading UI

No change. `use_resource` already yields `None` until the service resolves, and the views
render the existing `Spinner` in that state. With redb populated, the L2 read is a fast
synchronous disk hit, so the spinner barely flashes; a sustained wheel appears only on a true
cold start (no L1 and no L2), which is the intended behavior.

## Out of Scope

- **Web / WASM target.** The existing services already rely on `std::thread` + a tokio
  runtime (non-WASM), and redb is likewise a native file database. This pilot targets the
  current default (`mobile`) and desktop builds and changes nothing for web.
- **Cache invalidation UI / manual refresh.** Not part of this pass.
- **Migrating events/bots/traders.** Fast-follow after the items pilot validates the helper.

## Risks / Notes

- **redb 4 API drift:** v4 is a recent major; transaction/table method names verified at plan
  time against current docs.
- **Cross-platform write path on mobile** is the main unknown; resolved before the pilot ships.
- **Serialization stability:** if an `arc_api_rs` model changes shape, an old envelope may fail
  to deserialize — handled as a soft miss (re-fetch from API), so no crash.
