# Arc Page & Events Page — Design Spec

**Date:** 2026-05-22
**Status:** Approved (pending spec review)
**Author:** Dakota-LM (with Claude)

## Summary

Add two new pages to the Arc Raiders Companion App, following the existing
Materials/`ItemsView` pattern:

- **Arc page** — a searchable, name-sortable list of bots ("Arcs") using the
  same expandable card UX as Materials.
- **Events page** — a list of scheduled events with a **live countdown** driven
  by a local clock and a periodic API re-sync. Active events render at full
  opacity, upcoming events are visually faded, expired events are dropped.

Both pages reuse the existing card styling, grid layout, and data-service
conventions already established for Materials and Traders.

## Context / Current State

- **Framework:** Dioxus 0.7.3 (router feature). `default = ["mobile"]` — this is
  a **native** app (desktop/mobile); `web`/WASM is not a current target.
- Services already use `tokio` (rt-multi-thread), `std::thread::spawn`, and
  `moka` (sync) with a 15-minute cache TTL. `redb` is present for persistence.
- **Already wired (no changes needed):**
  - Routes in `src/main.rs`: `#[route("/")] Events {}` and `#[route("/arcs")] Arcs {}`.
  - Navbar links + icons in `src/views/navbar.rs` (`ICON_EVENTS`, `ICON_ARCS`,
    `events.svg`, `arcs.svg`).
  - `src/views/arcs.rs` and `src/views/events.rs` exist as 13-line stubs that
    wrap `PageLayout { title: "..." }` with no body component.
- **CSS convention:** each component declares
  `const X_CSS: Asset = asset!("/assets/styling/x.css");` and renders
  `document::Link { rel: "stylesheet", href: X_CSS }` inside its `rsx!`.
  Styling is plain CSS using `--color-*` variables from `common.css`.

### Data layer (`arc_api_rs` 0.2.x)

**Bot (Arc)** — `arc_api_rs::models::Bot`:

```rust
pub struct Bot {
    pub id: String,
    pub name: String,
    pub description: Option<Vec<String>>, // string-or-array; paragraphs
    pub icon: Option<UriString>,
    pub image: Option<UriString>,
    pub created_at: DateTimeString,
    pub updated_at: Option<DateTimeString>,
}
```

- Fetch: `MetaForgeClient::bots_all(&BotsQuery) -> Result<Vec<Bot>, MetaForgeError>`.
- `BotsQuery { page, limit, id, search, include_loot, sort_by, sort_order }`.
- No rarity/value/type/loot is deserialized onto `Bot` in 0.2.x — so the only
  "extra detail" to reveal on expand is the larger `image` + full `description`.

**ScheduledEvent** — `arc_api_rs::models::ScheduledEvent`:

```rust
pub struct ScheduledEvent {
    pub name: String,
    pub map: String,
    pub icon: String,       // URL string
    pub start_time: i64,    // epoch milliseconds
    pub end_time: i64,      // epoch milliseconds
}
// wrapper:
pub struct EventsScheduleResponse { pub data: Vec<ScheduledEvent>, pub cached_at: i64 }
```

- Fetch: `MetaForgeClient::events_schedule() -> Result<EventsScheduleResponse, MetaForgeError>`.

## Goals

1. Arc page: list bots; filter by name (search box); sort name A–Z / Z–A;
   expandable cards revealing image + description.
2. Events page: list events with a live, second-by-second countdown; fade
   upcoming events; hide expired events; periodically re-sync times from the API.
3. Reuse existing visual + service patterns; do not regress Materials/Traders.

## Non-Goals (YAGNI)

- Web/WASM support for the clock (native-only assumed; documented as a future
  cfg-gated change if web is prioritized).
- Server clock-skew correction (use the device clock as "now").
- Bot loot/drops display (not available on the `Bot` model).
- Filtering beyond name search on the Arc page; any sorting on the Events page
  beyond the fixed active-first / start-time ordering.

## Architecture

### File inventory

New files:

| File | Purpose |
|------|---------|
| `src/services/bots.rs` | Fetch bots via `MetaForgeClient`, moka cache, result struct. |
| `src/services/events.rs` | Fetch events schedule, moka cache, result struct. |
| `src/components/arc_card.rs` | Expandable bot card (mirrors `item_card.rs`). |
| `src/components/arcs_view.rs` | Arc list: search + sort + fetch + expand state. |
| `src/components/event_card.rs` | Single event card with live countdown + fade. |
| `src/components/events_view.rs` | Event list: clock tick, refetch loop, partition. |
| `assets/styling/arc_card.css` | Arc card styles (clone of `item_card.css`). |
| `assets/styling/arcs_view.css` | Arc list controls (search + sort toggle). |
| `assets/styling/event_card.css` | Event card layout + `--upcoming` opacity. |
| `assets/styling/events_view.css` | Event list layout (may reuse items grid). |

Modified files:

| File | Change |
|------|--------|
| `src/views/arcs.rs` | Render `ArcsView {}` inside `PageLayout`. |
| `src/views/events.rs` | Render `EventsView {}` inside `PageLayout`. |
| `src/services/mod.rs` | `mod bots; mod events;` + re-exports. |
| `src/components/mod.rs` | `mod`/`pub use` for the 4 new components. |

### Service layer (mirror `src/services/items.rs`)

Both services follow the existing convention:

- A lazily-initialized shared `MetaForgeClient` (reuse the same `HTTP_CLIENT` /
  client construction the items/traders services use).
- A `moka::sync::Cache` with `CACHE_TTL_SECS = 900` (15 min).
- A result struct carrying data + provenance, e.g.:

```rust
pub struct BotsResult {
    pub bots: Vec<Bot>,
    pub source: DataSource,      // Api | Cache | Fallback (reuse existing enum)
    pub error: Option<String>,
}

pub struct EventsResult {
    pub events: Vec<ScheduledEvent>,
    pub cached_at: i64,
    pub source: DataSource,
    pub error: Option<String>,
}
```

- An async fetch function used from `use_resource`, returning the result struct
  and never panicking (errors captured in `error`, `source = Fallback`).
- `bots.rs` builds `BotsQuery { search, sort_by: Some("name".into()),
  sort_order: Some("asc"|"desc"), ..Default::default() }`. Search and sort may be
  applied client-side as well to keep the UI responsive against the cache.

> Reuse the exact caching/threading idiom already in `items.rs`/`traders.rs`
> (including the `std::thread::spawn` cache-population pattern) rather than
> inventing a new one.

### Arc page

**`arc_card.rs`** — clone `item_card.rs`:

- Props: the `Bot`, `expanded: bool`, an `on_toggle` callback (id-based), mirroring
  how `ItemCard` receives `expanded_id` semantics.
- Summary row: `icon` (3rem, fallback if `None`) + `name`.
- Details section: `class: "arc-card__details"` → `"arc-card__details--open"` when
  expanded; contains the larger `image` and each `description` paragraph as its
  own `<p>`. Reuse the `item-detail-fade-in` animation (duplicate the keyframe in
  `arc_card.css` or reference the shared one).
- Rarity border coloring is **not** applicable; use a neutral border.

**`arcs_view.rs`** — clone `items_view.rs` (simplified):

- State: `expanded_id: Signal<Option<String>>`, `search: Signal<String>`,
  `sort_desc: Signal<bool>` (A–Z default).
- `use_resource` fetches the full `BotsResult` once (cached). Search and sort are
  applied **client-side** over the cached `Vec<Bot>` so typing/toggling is instant
  and does not re-hit the network. (`BotsQuery.search`/`sort_by` are still set on
  the initial fetch but the live UI filtering is local.)
- Controls bar: a text input (search) + an A–Z / Z–A toggle button
  (reuse `toggle.rs` or the items-view toggle-button styling).
- Renders a list of `ArcCard`s in the `items_view.css` grid; clicking a card
  toggles `expanded_id` (single-open behavior, same as Materials).
- Loading → `Spinner`; error/empty → a friendly message; optional data-source
  debug banner consistent with the items view.

### Events page

**`events_view.rs`** — owns timing + data:

- `now: Signal<i64>` initialized to `SystemTime::now()` epoch ms.
- `refresh: Signal<u32>` (refetch counter), initial `0`.
- A `use_future` clock loop:
  ```text
  loop {
      tokio::time::sleep(Duration::from_secs(1)).await;
      now.set(current_epoch_ms());
      tick += 1;
      if tick % 60 == 0 { refresh += 1; }   // ~every 60s
  }
  ```
  (One loop drives both the 1s display tick and the 60s refetch trigger.)
- `use_resource` reads `refresh` and calls `events::fetch(...)` → `EventsResult`.
  When new data arrives, the authoritative `start_time`/`end_time` replace the
  prior values; the local clock keeps ticking between fetches.
- Derivation (recomputed reactively against `now`):
  - **expired:** `end_time <= now` → **excluded** (no card).
  - **active:** `start_time <= now < end_time` → full opacity.
  - **upcoming:** `now < start_time` → faded.
  - **Sort:** active events first, ordered by `end_time` ascending (soonest to
    end = most urgent, on top); then upcoming events ordered by `start_time`
    ascending (soonest to begin on top).
- Renders `EventCard`s; loading → `Spinner`; empty (no active/upcoming) → message.

**`event_card.rs`**:

- Props: the `ScheduledEvent`, `now: i64`, and a derived `state`
  (`Active` | `Upcoming`).
- Layout: `icon` + `name` + `map` + countdown line.
- Countdown text from `now` vs start/end:
  - Active: `"Ends in {hms}"` where remaining = `end_time - now`.
  - Upcoming: `"Starts in {hms}"` where remaining = `start_time - now`.
  - Format helper: ms → compact `Hh Mm` / `Mm Ss` (e.g. `1h 23m`, `4m 09s`).
- Class: base `event-card`; add `event-card--upcoming` when upcoming
  → CSS sets `opacity: ~0.55`.

### CSS

- `arc_card.css` — copy `item_card.css`, drop rarity-border rules, keep the
  summary/details/`--open` structure and fade-in keyframe.
- `event_card.css` — flex row (icon, name, map, countdown);
  `.event-card--upcoming { opacity: 0.55; }`; inherit `--color-*` vars.
- `arcs_view.css` / `events_view.css` — controls bar + list spacing; reuse the
  `items_view.css` grid where possible (import or duplicate minimal rules).

## Data Flow

```
ArcsView ──use_resource──▶ services::bots::fetch ──▶ moka cache / MetaForgeClient
   │ (search, sort applied client-side)
   └─▶ ArcCard × N (expanded_id controls single-open)

EventsView
   ├─ use_future: 1s tick → now.set(); every 60th tick → refresh += 1
   ├─ use_resource(refresh) ──▶ services::events::fetch ──▶ moka / events_schedule()
   └─ partition by (now vs start/end): drop expired, fade upcoming, show active
        └─▶ EventCard × N (reads now → live countdown)
```

## Error Handling

- Network/API failure: services return `source = Fallback`, `error = Some(msg)`,
  empty data; views show a non-fatal message and (optionally) a debug banner.
  No panics, consistent with items/traders services.
- Missing `icon`/`image` on a `Bot`: render a placeholder/skip the `<img>`.
- Empty `description`: render no detail paragraphs (card still expands cleanly or
  shows a "No description" note).
- Clock: device-clock based; no skew correction. If `now` jumps (device clock
  change), the next render simply reflects it.

## Testing / Verification

- `cargo check` (default features) passes; no new warnings in the new modules.
- Run the app (`dx serve` / project run skill) and confirm:
  - **Arc:** search filters by name; A–Z/Z–A reorders; a card expands to show
    image + description and collapses; single-open behavior holds.
  - **Events:** countdowns ticking every second; upcoming events faded; expired
    events absent; after ~60s a refetch occurs and times stay consistent;
    a card that crosses its start boundary flips from faded→full opacity live.
- Manual cross-check that Materials/Traders pages still render unchanged.

## Open Questions / Risks

- **Web/WASM:** explicitly out of scope. The 1s loop uses `tokio::time::sleep`
  and `std::time::SystemTime`; both are native. A future web build needs
  cfg-gated `gloo-timers` + `js_sys::Date`. Documented, not implemented.
- **Refetch cost:** 60s refetch hits a server cache (15-min) most of the time —
  cheap; it mainly catches schedule changes / newly-active events. Interval is a
  single named constant for easy tuning.
- **Resolved decisions:** active events sort by `end_time` asc; Arc search/sort
  are client-side over the cached list; refetch interval = 60s.
