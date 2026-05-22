# Events Page UI Tweaks — Design Spec

**Date:** 2026-05-22
**Status:** Approved (pending spec review)
**Branch:** `feat/arc-and-events-pages`
**Author:** Dakota-LM (with Claude)

## Summary

Three UI refinements to the Events page (already shipping `EventsView` +
`EventCard`):

1. **Uniform card size** — every event card is the same height.
2. **Layout priority** — the event name and map never wrap to a new line; the
   countdown timer is the element that gives way (wraps) to make room.
3. **Filters** — multi-select chip filters for **Map** and **Event Type**
   (event type = the event's `name`).

## Context / Current State

- `src/components/event_card.rs` — `EventCard` renders a flex row:
  `icon` + `info(name, map)` + `countdown`. Today `info` has `flex: 1` and the
  countdown is `white-space: nowrap`; long names/maps wrap to extra lines, which
  is what makes card heights non-uniform.
- `src/components/events_view.rs` — `EventsView` fetches via
  `get_event_schedule()`, runs a 1s clock + 60s refetch loop, then
  `partition_events` (drop expired, active-first) and `event_render_keys`
  (unique keys), and renders the list.
- `arc_api_rs::models::ScheduledEvent` fields: `name`, `map`, `icon`,
  `start_time`, `end_time`. **No dedicated "type" field** — "event type" = `name`.
- Existing chip styling lives in `assets/styling/filter_chips.css`; the
  active-chip highlight pattern is `items-view__toggle-btn--active`
  (accent background). `FilterChips` itself bundles a search box + sort dropdown +
  add-via-dropdown flow, so it is **not** reused here.

## Goals

1. All event cards render at a uniform height.
2. `name` and `map` each stay on a single line (never wrap); the countdown wraps
   / gives way to accommodate them.
3. Two multi-select chip filters (Map, Event Type) restrict the visible events.

## Non-Goals (YAGNI)

- No search box or sort controls on the Events page.
- No new API fields / no `arc_api_rs` bump (event type is `name`).
- No font-scaling of the timer (we wrap it; scaling was offered and declined in
  favor of wrapping).
- No persistence of selected filters across app restarts.
- No changes to the Arc/Materials/Traders pages.

## Architecture

### File inventory

New files:

| File | Purpose |
|------|---------|
| `src/components/event_filters.rs` | `EventFilters` chip component (Maps + Event Types groups). |
| `assets/styling/event_filters.css` | Chip group + chip + active-chip styling. |

Modified files:

| File | Change |
|------|--------|
| `assets/styling/event_card.css` | name/map nowrap; countdown wraps + gives way; card `min-height`. |
| `src/components/events_view.rs` | Add `distinct_maps`/`distinct_types`/`filter_events` helpers + tests; add `selected_maps`/`selected_types` signals; insert filtering into the pipeline; render `EventFilters`. |
| `src/components/mod.rs` | Register `mod event_filters; pub use event_filters::EventFilters;`. |

### 1. Card layout + uniform size (`event_card.css`)

- `.event-card__name`, `.event-card__map`:
  `white-space: nowrap; overflow: hidden; text-overflow: ellipsis;`
  — single line each; ellipsis is a graceful last resort only.
- `.event-card__info`: `flex: 1 1 auto; min-width: 0;` — holds name/map at
  priority width; it shrinks (and ellipsis-truncates) only as an absolute last
  resort, i.e. after the countdown has fully given way.
- `.event-card__countdown`: becomes the flexible element that yields **first** —
  `white-space: normal; text-align: right;` (remove the existing
  `white-space: nowrap`) plus a much higher `flex-shrink` than `info` (e.g.
  `flex: 0 100 auto`), so when the row is narrow the countdown wraps to a second
  line before name/map are ever compressed. Implementer tunes the flex factors
  so the timer demonstrably yields before name/map.
- `.event-card`: add `min-height` (≈ icon 3rem + vertical padding, e.g.
  `4.5rem`) so all cards are equal height. With name/map no longer wrapping, the
  icon governs height uniformly.

### 2. Pure filter helpers (`events_view.rs`, TDD)

```rust
/// Distinct map names across the events, sorted ascending, de-duplicated.
fn distinct_maps(events: &[ScheduledEvent]) -> Vec<String>

/// Distinct event types (== event `name`), sorted ascending, de-duplicated.
fn distinct_types(events: &[ScheduledEvent]) -> Vec<String>

/// Keep events matching the selected maps AND the selected types.
/// An empty selection for a group imposes no constraint on that group.
/// Within a group, membership is OR; across the two groups it is AND.
fn filter_events(
    events: &[ScheduledEvent],
    selected_maps: &[String],
    selected_types: &[String],
) -> Vec<ScheduledEvent>
```

`filter_events` predicate per event `e`:
`(selected_maps.is_empty() || selected_maps.contains(&e.map))
 && (selected_types.is_empty() || selected_types.contains(&e.name))`.

Distinct lists are derived from **all fetched events** (not just visible), so the
chip set is stable as events transition/expire.

### 3. `EventFilters` component (`event_filters.rs`)

A focused, stateless chip component:

```rust
#[component]
pub fn EventFilters(
    maps: Vec<String>,            // all distinct maps
    types: Vec<String>,           // all distinct event types (names)
    selected_maps: Vec<String>,
    selected_types: Vec<String>,
    on_toggle_map: EventHandler<String>,
    on_toggle_type: EventHandler<String>,
) -> Element
```

- Two labeled groups ("Maps", "Event Types"); each value is a tappable chip.
- A chip is rendered active (accent background, mirroring
  `items-view__toggle-btn--active`) when its value is in the corresponding
  `selected_*` list; tapping calls `on_toggle_*` with the value.
- Chips wrap (chip cloud) so long type lists flow onto multiple rows.
- If a group has no values it renders nothing.

### `EventsView` integration

Add state and rewire the render pipeline:

```rust
let mut selected_maps = use_signal(Vec::<String>::new);
let mut selected_types = use_signal(Vec::<String>::new);
// ...existing now/refresh signals, clock loop, resource...

let maps = distinct_maps(&all);
let types = distinct_types(&all);
let sel_maps = selected_maps();
let sel_types = selected_types();
let filtered = filter_events(&all, &sel_maps, &sel_types);
let visible = partition_events(&filtered, now_val);
let render_keys = event_render_keys(&visible);
```

Toggle handlers add/remove a value from the relevant signal's `Vec`.
`EventFilters` renders above `.events-view__list`. The empty-state message logic
stays, but should distinguish "no events at all" from "no events match the
selected filters" (e.g. when `all` is non-empty but `visible` is empty due to
filters, show "No events match the selected filters.").

## Data Flow

```
fetch (get_event_schedule)
  → all: Vec<ScheduledEvent>
  → distinct_maps(all), distinct_types(all)         ── feed EventFilters chips
  → filter_events(all, selected_maps, selected_types)
  → partition_events(filtered, now)                 ── drop expired, active-first
  → event_render_keys(visible)                       ── unique keys
  → render EventFilters + list of EventCard
```

## Error / Edge Handling

- No events fetched (`all` empty): existing "Failed to load events." message.
- Events fetched but none active/upcoming after filtering: "No active or
  upcoming events." (no filters) vs "No events match the selected filters."
  (filters active).
- Selecting a map/type whose events are all expired yields the filtered-empty
  message — acceptable.
- Pathologically long name/map: ellipsis truncation (never wraps).

## Testing / Verification

- TDD unit tests in `events_view.rs`:
  - `distinct_maps` / `distinct_types`: dedup + sorted ascending.
  - `filter_events`: empty selection = all; OR within map group; OR within type
    group; AND across groups; map-only and type-only selections.
- `cargo test --no-default-features --features desktop` (full suite) passes;
  `cargo check --no-default-features --features desktop` clean.
- Manual Android run (`dx serve --platform android`): cards are uniform height;
  long names/maps stay on one line while the timer wraps; tapping Map/Type chips
  filters the list (multi-select; OR within group, AND across); clearing
  selections restores the full list.

## Decisions

- Event type = event `name`.
- Filters are multi-select tappable chips (focused `EventFilters`, not the
  items `FilterChips`).
- Timer wraps (not font-scaled).
- Distinct chip values derived from all fetched events.
