# Tier-Aware Cache Badge + Settings Clear-Cache — Design

**Date:** 2026-05-22
**Status:** Approved (design)
**Builds on:** `2026-05-22-redb-l2-cache-design.md` (the Moka L1 / redb L2 cache this surfaces).

## Problem

The app caches data in two tiers (Moka in-memory L1, redb on-disk L2) with an API source, but there is no clear way to tell, while using the app, *whether* the cache is working or *which* tier served the data. Today:

- Each data service computes a `source` (currently `Api` / `Cache`, plus `Fallback` for traders) and a `count`, but only two pages surface it: `items_view` and `trader_view` show a "debug banner" built from a formatted string and selected via brittle substring matching (e.g. `text.contains("Source: API")`). `arcs_view` and `events_view` discard the metadata entirely.
- The single `Cache` value cannot distinguish a Moka (memory) hit from a redb (disk) hit, so even where a badge exists it can't show which cache.
- There is no way to clear the caches from the UI.

## Goals

1. A consistent, tier-aware source badge on all four data pages (materials/items, traders, arcs/bots, events).
2. Distinguish the tiers: **API**, **Memory** (Moka L1), **Disk** (redb L2), **Fallback** (traders' hardcoded list).
3. A Settings → Advanced → **Clear cache** control that flushes both Moka and redb for all services.

## Decisions (from brainstorming)

- **Granularity:** four states — `API` / `Memory` / `Disk` / `Fallback`. Fresh and stale redb both report `Disk` (no separate "stale" state).
- **Structure:** one shared `CacheSource` enum + one reusable `CacheBadge` component, replacing the four per-service `DataSource` enums and the two inline string-matching banners.
- **Clear-cache UX:** one tap → clears all caches → brief inline "Cache cleared" confirmation that auto-resets after ~2s. No modal.

## Components

### 1. `CacheSource` enum — `src/services/source.rs` (new)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSource {
    Api,      // fetched live from the MetaForge API
    Memory,   // served from the Moka in-memory L1 cache
    Disk,     // served from the redb on-disk L2 cache (fresh or stale)
    Fallback, // traders' hardcoded fallback list (API + caches all unavailable)
}
```

- `pub fn label(&self) -> &'static str` → `"API" | "Memory" | "Disk" | "Fallback"`.
- `pub fn css_class(&self) -> &'static str` → `"api" | "memory" | "disk" | "fallback"`.
- Registered via `pub mod source;` in `services/mod.rs`. `Copy` + fieldless, so it drops cleanly into the `RefCell<Option<CacheSource>>` the loaders use (no clone friction).

This **replaces** `items::DataSource`, `events::DataSource`, `bots::DataSource`, and `traders::DataSource` (and their `Display` impls). Each `*Result.source` field becomes `CacheSource`.

### 2. `CacheBadge` component — `src/components/cache_badge.rs` + `assets/styling/cache_badge.css` (new)

```rust
#[component]
pub fn CacheBadge(
    source: CacheSource,
    #[props(default)] count: Option<usize>,
    #[props(default)] label: Option<String>,
    #[props(default)] error: Option<String>,
) -> Element
```

- Renders a chip: `class="cache-badge cache-badge--{source.css_class()}"`.
- Text: optional `{label}: ` prefix, then `{source.label()}`, then `· {count}` when `count` is `Some`. When `error` is `Some`, the message is appended (e.g. `· {error}`) and the chip uses a warning treatment.
- Exported from `components/mod.rs` as `CacheBadge`.
- CSS defines `.cache-badge` plus modifiers `--api`, `--memory`, `--disk`, `--fallback`, using existing theme CSS variables for light/dark parity. Colors carry over the spirit of the current debug-banner palette (API vs cache vs fallback distinct).

### 3. Service wiring — `items.rs`, `bots.rs`, `events.rs`, `traders.rs`

Replace each `DataSource` with `CacheSource` and set the tier at each cascade branch:

| Cascade branch | `CacheSource` |
|---|---|
| Moka L1 hit (`!entry.is_fresh()`, or the manual L1 `get` in traders) | `Memory` |
| Loader/flow served fresh redb | `Disk` |
| Loader/flow served stale redb (offline fallback) | `Disk` |
| API fetch succeeded | `Api` |
| Traders' hardcoded fallback list | `Fallback` |
| Total failure with no data (error result) | `Api` (unchanged; error field populated) |

For items/bots/events the `RefCell<Option<DataSource>>` becomes `RefCell<Option<CacheSource>>`; the `!is_fresh()` arm yields `Memory`; the loader sets `Disk` (fresh/stale redb) or `Api`. Traders sets `CacheSource` directly at each `return`.

Add **`invalidate_events_cache()`** to `events.rs` (mirroring the others): `EVENTS_CACHE.invalidate(...)` + `db::remove(EVENTS_TABLE, EVENTS_CACHE_KEY)`, marked `#[allow(dead_code)]` until wired.

### 4. Badges on all four data pages

- **`items_view.rs` (refactor):** drop `debug_info: Signal<String>` and `banner_class` substring logic; capture the result's `source`/`count`/`error` into signals and render `CacheBadge` in the same spot (the `items-debug` slot).
- **`trader_view.rs` (refactor):** replace the two inline banners with two `CacheBadge`s, `label: "Names"` (no count) and `label: "Items"` (with count), driven by the real `CacheSource`. Existing 📡/📦 emoji are dropped for a unified look.
- **`arcs_view.rs` (add):** capture `get_all_bots()`'s `source`/`count` into signals and render `CacheBadge` above/near the list.
- **`events_view.rs` (add):** capture `get_event_schedule()`'s `source`/`count` into signals (preserving the existing 60s refresh loop) and render `CacheBadge`. The badge reflects the most recent fetch.

### 5. Settings → Advanced → Clear cache — `views/settings.rs`

- Add an "Advanced" section below the existing controls.
- A **Clear cache** button whose `onclick`:
  1. Calls `invalidate_items_cache()`, `invalidate_bots_cache()`, `invalidate_events_cache()`, `invalidate_trader_cache()` (each already flushes Moka + redb).
  2. Sets a `cleared: Signal<bool>` (or a status string) to show an inline "Cache cleared" message.
  3. `spawn`s a task that sleeps ~2s and resets the message.
- Button styled to match the Settings page; cohesive with existing controls.

## Data Flow

`use_resource` calls `get_*()` → `*Result { source: CacheSource, count, .. }` → view stores `source`/`count` in signals → `CacheBadge` renders the tier chip. Clearing in Settings invalidates both tiers, so the next page load re-runs the cascade from the API and the badge shows `API`.

## Testing

- **TDD:** `CacheSource::label()` and `css_class()` (pure, exhaustive over variants).
- **Build/lint gates:** `cargo test --no-default-features --features desktop`, `cargo clippy --no-default-features --features desktop` (no new warnings in changed files). The `mobile` default feature does not link on a host.
- **Manual:** on the Android emulator — confirm each page shows a badge, that it reads `API` on a cold fetch and `Memory`/`Disk` on subsequent loads, and that Settings → Clear cache shows the confirmation and forces an `API` refetch next visit.

## Out of Scope

- A separate "Disk (stale)" state (folded into `Disk`).
- Showing cache file size / entry counts or per-tier stats in Settings.
- Confirmation modal for Clear cache (one-tap + inline confirmation only).
- Any change to the cascade behavior itself (this only surfaces and clears it).

## Risks / Notes

- Replacing the per-service `DataSource` enums touches all four services and both existing badges; mitigated by doing it as a focused enum swap with the build as the gate.
- The old `items-debug-banner` / `trader-debug-banner` CSS becomes dead once both views migrate; remove it as part of the view refactor.
- Dioxus component props require `PartialEq` + `Clone`; `CacheSource` derives both (plus `Copy`).
