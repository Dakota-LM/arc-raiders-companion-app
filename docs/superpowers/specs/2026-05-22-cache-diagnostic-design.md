# Dev-Only Cache-State Diagnostic — Design

**Date:** 2026-05-22
**Status:** Approved (design)
**Builds on:** `2026-05-22-cache-indicator-design.md` (the tier-aware `CacheBadge` this extends) and `2026-05-22-redb-l2-cache-design.md` (the Moka L1 / redb L2 cache).

## Problem

The current per-page `CacheBadge` shows only the tier that *served* a load. Because the cascade checks Moka (L1) first, once both caches are warm every load is served from L1 — so in practice the badge almost always reads "Memory". "Disk" only appears in the narrow window where L1 expired but redb is still fresh, and "API" only on a true cold fetch. The badge therefore can't show that the L2 (redb) cache is working, nor the live state of each tier. It also renders in all builds, when it is only useful during development.

## Goals

1. Show, per load, the **served tier** *plus* the **state of both caches** for that key: L1 (Moka) Hit/Miss and L2 (redb) Fresh/Stale/Miss.
2. The states reflect the moment **just before** the load, so they explain *why* it served from that tier (e.g. cold = L1 miss + L2 miss → API; after restart = L1 miss + L2 fresh → Disk).
3. Render the diagnostic **only in development/debug builds** (`dx serve` / emulator), never in a shipped release build.

## Decisions (from brainstorming)

- **Layout:** "served chip + two state pills" — `[Served] L1:[state] L2:[state]`, three pills in a row, each colored by its own state.
- **Gating:** runtime `cfg!(debug_assertions)` at both the probe call and the render site. The code exists in all builds but is inert (and renders nothing) in release.
- **Semantics:** pre-load probe (the chosen layout's example states — cold `API/miss/miss`, warm `Memory/hit/fresh`, restart `Disk/miss/fresh` — are exactly the pre-load reading).
- **Architecture:** a *decoupled, read-only probe* — the working cascade and `get_*()` are NOT modified. `source` already supplies the served tier; a separate probe supplies the L1/L2 states.

## Components

### 1. New types — `src/services/source.rs`

Alongside `CacheSource`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L1State { Hit, Miss }      // Moka, for this key

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L2State { Fresh, Stale, Miss } // redb, for this key

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheState { pub l1: L1State, pub l2: L2State }
```

- `L1State::label()` → `"hit"|"miss"`; `css_class()` → `"hit"|"miss"`.
- `L2State::label()` → `"fresh"|"stale"|"miss"`; `css_class()` → `"fresh"|"stale"|"miss"`.
- (Labels are lowercase to match the pill text `L1:[hit]`; the badge component prefixes `L1:`/`L2:`.)

### 2. Lightweight L2 probe — `src/services/db.rs`

```rust
pub fn l2_state(table: TableDefinition<&str, &[u8]>, key: &str, ttl: Duration) -> L2State
```

Reads the raw envelope bytes for `key` and deserializes **only** a `{ cached_at: i64 }` helper struct (serde ignores the `data` field), so it is type-agnostic and never deserializes the payload. Classifies:
- `Fresh` — entry present and `now - cached_at <= ttl`.
- `Stale` — entry present but older than `ttl`.
- `Miss` — key absent, table absent, or bytes unreadable (soft).

This reuses the same envelope shape `db::write` produces; any read/parse error is treated as `Miss`.

### 3. Read-only probe functions per service

In each service, a sync probe that does NOT touch the cascade and does NOT mutate either cache:

| Service | Function | L1 key (Moka) | L2 key (redb table) |
|---|---|---|---|
| items | `items_cache_state()` | `ITEMS_CACHE_KEY` | `ITEMS_TABLE` / `ITEMS_CACHE_KEY` |
| bots | `bots_cache_state()` | `BOTS_CACHE_KEY` | `BOTS_TABLE` / `BOTS_CACHE_KEY` |
| events | `events_cache_state()` | `EVENTS_CACHE_KEY` | `EVENTS_TABLE` / `EVENTS_CACHE_KEY` |
| traders | `trader_names_cache_state()` | `TRADER_NAMES_KEY` | `TRADER_NAMES_TABLE` / `TRADER_NAMES_KEY` |
| traders | `trader_items_cache_state(name: &str)` | `format!("{TRADER_ITEMS_PREFIX}{name}")` | `TRADER_ITEMS_TABLE` / bare `name` |

Each returns `CacheState { l1: if CACHE.contains_key(&key) { Hit } else { Miss }, l2: db::l2_state(TABLE, key, Duration::from_secs(CACHE_TTL_SECS)) }`. `moka::sync::Cache::contains_key` is non-mutating (verified), so probing does not perturb LRU/eviction. These fns are not `#[allow(dead_code)]`: the views call them inside `if cfg!(debug_assertions)`, which the compiler still sees as a call (runtime gate, code compiled in all builds).

### 4. `CacheDiagnostic` component — `src/components/cache_diagnostic.rs`

```rust
#[component]
pub fn CacheDiagnostic(
    source: CacheSource,
    state: CacheState,
    #[props(default)] count: Option<usize>,
    #[props(default)] label: Option<String>,
    #[props(default)] error: Option<String>,
) -> Element
```

Renders a row: the existing `CacheBadge { source, count, label, error }` (the served chip, reused unchanged) followed by two pills:
- `L1:` + `L1State` pill, class `cache-pill cache-pill--{l1.css_class()}`.
- `L2:` + `L2State` pill, class `cache-pill cache-pill--{l2.css_class()}`.

Pill styles (`cache-pill`, `--hit`/`--miss`/`--fresh`/`--stale`, + `:root.light` variants) are added to `assets/styling/cache_badge.css`. `hit`/`fresh` use the green/blue "good" tones; `miss` uses a muted/neutral tone; `stale` uses the amber warning tone.

### 5. View wiring (all four data pages)

In each view's resource closure, before calling `get_*()`:

```rust
let probe = if cfg!(debug_assertions) { Some(<svc>_cache_state()) } else { None };
let result = get_*().await;
// store result.source/count/error AND probe into signals
```

A `cache_state: Signal<Option<CacheState>>` holds the probe. Render, replacing the current bare `CacheBadge`:

```rust
if cfg!(debug_assertions) {
    if let Some(state) = cache_state() {
        CacheDiagnostic { source: data_source(), count: Some(data_count()), error: data_error(), state }
    }
}
```

(kept inside the existing `if !loading { … }` guard / wrapper). **traders** renders two `CacheDiagnostic`s — Names (`trader_names_cache_state()`, no count, label `"Names"`) and Items (`trader_items_cache_state(&trader_name)`, with count, label `"Items"`).

### 6. Gating

`cfg!(debug_assertions)` (runtime) guards both the probe call and the render. In a release build (`dx build --release`) the probe is never called and the diagnostic never renders — no diagnostic UI ships and no extra redb reads occur. `CacheBadge` remains as the served-pill building block, now only ever rendered inside `CacheDiagnostic`.

## Data Flow

`use_resource` → (debug only) `<svc>_cache_state()` reads Moka `contains_key` + `db::l2_state` for the pre-load snapshot → `get_*()` runs the normal cascade and returns `source`/`count`/`error` → view stores both → `CacheDiagnostic` renders served chip + L1 pill + L2 pill (debug only).

## Testing

- **TDD:** `L1State`/`L2State` `label()`/`css_class()` (exhaustive); `db::l2_state` against a temp redb — fresh (recent `cached_at`), stale (old `cached_at`), miss (absent key), reusing the existing `db.rs` test harness.
- **Build/lint gates:** `cargo test --no-default-features --features desktop`; `cargo clippy --no-default-features --features desktop` (no new warnings in changed files).
- **Manual (Android emulator, dev build):** confirm each page shows `[Served] L1:[…] L2:[…]`; cold = `API / miss / miss`; reload = `Memory / hit / fresh`; after app restart = `Disk / miss / fresh`; after Settings → Clear cache = `API / miss / miss`. A `--release` build shows no diagnostic.

## Out of Scope

- Post-load cache state (we show pre-load, which explains the served tier).
- Compile-time `#[cfg]` removal (runtime `cfg!` chosen; release renders nothing and the gated calls are inert).
- Any change to the cascade, `get_*()` behavior, or the `Settings → Clear cache` control.
- Showing the diagnostic in release builds.

## Risks / Notes

- The probe adds two cheap reads (Moka `contains_key` + one redb transaction) per load **in debug only**; negligible and never in release.
- `db::l2_state`'s `{ cached_at }`-only deserialize relies on serde's default ignore-unknown-fields behavior (the envelope is not `deny_unknown_fields`) — true for the current `CacheEnvelope`.
- Pre-load semantics mean that immediately after a fetch the *next* render still shows the pre-load snapshot for that load; the snapshot updates on the next resource run (e.g. events' 60s refresh, navigating back, or Clear cache). This is the intended "why it served from X" reading.
