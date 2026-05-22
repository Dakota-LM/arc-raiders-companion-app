# Events Page UI Tweaks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Events cards a uniform height, keep event name/map on a single line while the countdown timer yields, and add multi-select Map + Event-Type chip filters.

**Architecture:** A CSS-only change to `event_card.css` for sizing/wrapping; pure, unit-tested helpers (`distinct_maps`, `distinct_types`, `filter_events`) in `events_view.rs`; a focused stateless `EventFilters` chip component; and `EventsView` wiring (selected-map/type signals + a fetch → filter → partition → render pipeline).

**Tech Stack:** Rust, Dioxus 0.7.3 (native, `default = ["mobile"]`), `arc_api_rs` 0.2.x. Plain CSS with `--color-*` variables.

**Conventions (verified against the codebase):**
- Test command: `cargo test --no-default-features --features desktop <filter>`
- Check command: `cargo check --no-default-features --features desktop`
- Manual run: `dx serve --platform android` (the user verifies GUI/visual behavior; agents must NOT run `dx serve` — it blocks).
- "Event type" == the event's `name` (the API exposes no separate type field).
- Component CSS pattern: `const X_CSS: Asset = asset!("/assets/styling/x.css");` + `document::Link { rel: "stylesheet", href: X_CSS }` in `rsx!`.
- Per-iteration node block pattern in `rsx!` (as used in `filter_chips.rs`): `for x in xs.iter() { { let v = x.clone(); rsx! { ... } } }`.

---

## Task 1: Card layout — uniform height, no-wrap name/map, yielding timer

**Files:**
- Modify: `assets/styling/event_card.css`

This is a CSS-only change; there is no unit test (visual behavior is verified manually on Android by the user). Agents verify only that the project still compiles.

- [ ] **Step 1: Replace the stylesheet contents**

Replace the entire contents of `assets/styling/event_card.css` with:

```css
.event-card {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 0.75rem;
  background-color: var(--color-bg-secondary);
  border: 0.0625rem solid var(--color-border);
  border-radius: 0.5rem;
  padding: 0.75rem;
  min-height: 4.5rem;
  transition: opacity 0.2s ease;
}

.event-card--upcoming {
  opacity: 0.55;
}

.event-card__icon {
  width: 3rem;
  height: 3rem;
  object-fit: contain;
  flex-shrink: 0;
}

.event-card__info {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-width: 0;
}

.event-card__name {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.event-card__map {
  font-size: 0.8rem;
  color: var(--color-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.event-card__countdown {
  flex: 0 100 auto;
  min-width: 0;
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-accent);
  white-space: normal;
  text-align: right;
}
```

Rationale: `min-height` makes all cards uniform; name/map get `nowrap` + ellipsis (single line, ellipsis only as an extreme fallback); the countdown's high `flex-shrink: 100` (vs the info block's `1`) means the timer wraps/yields first when the row is narrow.

- [ ] **Step 2: Verify the project still compiles**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add assets/styling/event_card.css
git commit -m "style(events): uniform card height; keep name/map on one line, timer yields"
```

---

## Task 2: Filter helpers — `distinct_maps`, `distinct_types`, `filter_events` (TDD)

**Files:**
- Modify: `src/components/events_view.rs`

These three pure functions are added to `events_view.rs` (alongside the existing `partition_events` / `event_render_keys`). Write the stubs + tests first (red), then implement (green).

- [ ] **Step 1: Add function stubs**

In `src/components/events_view.rs`, add these three functions immediately after the existing `event_render_keys` function (before the `#[component] pub fn EventsView`):

```rust
/// Distinct map names across the events, sorted ascending and de-duplicated.
fn distinct_maps(events: &[ScheduledEvent]) -> Vec<String> {
    unimplemented!()
}

/// Distinct event types (the event `name`), sorted ascending and de-duplicated.
fn distinct_types(events: &[ScheduledEvent]) -> Vec<String> {
    unimplemented!()
}

/// Keep events whose map is in `selected_maps` AND whose name is in
/// `selected_types`. An empty selection for a group imposes no constraint on
/// that group (OR within a group, AND across groups).
fn filter_events(
    events: &[ScheduledEvent],
    selected_maps: &[String],
    selected_types: &[String],
) -> Vec<ScheduledEvent> {
    unimplemented!()
}
```

- [ ] **Step 2: Add the failing tests**

In the existing `#[cfg(test)] mod tests` block in `src/components/events_view.rs`, add a fixture helper and tests (place them after the existing `ev` helper / alongside the other tests):

```rust
    fn event(name: &str, map: &str) -> ScheduledEvent {
        ScheduledEvent {
            name: name.to_string(),
            map: map.to_string(),
            icon: String::new(),
            start_time: 0,
            end_time: 1000,
        }
    }

    #[test]
    fn distinct_maps_sorted_and_deduped() {
        let evs = [event("A", "Dam"), event("B", "Spaceport"), event("C", "Dam")];
        assert_eq!(
            distinct_maps(&evs),
            vec!["Dam".to_string(), "Spaceport".to_string()]
        );
    }

    #[test]
    fn distinct_types_sorted_and_deduped() {
        let evs = [event("Storm", "Dam"), event("Boss", "Dam"), event("Storm", "Spaceport")];
        assert_eq!(
            distinct_types(&evs),
            vec!["Boss".to_string(), "Storm".to_string()]
        );
    }

    #[test]
    fn filter_empty_selection_returns_all() {
        let evs = [event("Storm", "Dam"), event("Boss", "Spaceport")];
        assert_eq!(filter_events(&evs, &[], &[]).len(), 2);
    }

    #[test]
    fn filter_by_map_only() {
        let evs = [event("Storm", "Dam"), event("Boss", "Spaceport"), event("Rush", "Dam")];
        let out = filter_events(&evs, &["Dam".to_string()], &[]);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|e| e.map == "Dam"));
    }

    #[test]
    fn filter_by_type_only() {
        let evs = [event("Storm", "Dam"), event("Storm", "Spaceport"), event("Boss", "Dam")];
        let out = filter_events(&evs, &[], &["Storm".to_string()]);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|e| e.name == "Storm"));
    }

    #[test]
    fn filter_or_within_group() {
        let evs = [event("Storm", "Dam"), event("Boss", "Spaceport"), event("Rush", "Buri")];
        let out = filter_events(&evs, &["Dam".to_string(), "Spaceport".to_string()], &[]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn filter_and_across_groups() {
        let evs = [
            event("Storm", "Dam"),
            event("Storm", "Spaceport"),
            event("Boss", "Dam"),
        ];
        let out = filter_events(&evs, &["Dam".to_string()], &["Storm".to_string()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Storm");
        assert_eq!(out[0].map, "Dam");
    }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --no-default-features --features desktop events_view::tests`
Expected: the 7 new tests FAIL / panic with `not implemented` (the existing partition/key tests still pass).

- [ ] **Step 4: Implement the three functions**

Replace the three stub bodies in `src/components/events_view.rs`:

```rust
fn distinct_maps(events: &[ScheduledEvent]) -> Vec<String> {
    let mut maps: Vec<String> = events.iter().map(|e| e.map.clone()).collect();
    maps.sort();
    maps.dedup();
    maps
}

fn distinct_types(events: &[ScheduledEvent]) -> Vec<String> {
    let mut types: Vec<String> = events.iter().map(|e| e.name.clone()).collect();
    types.sort();
    types.dedup();
    types
}

fn filter_events(
    events: &[ScheduledEvent],
    selected_maps: &[String],
    selected_types: &[String],
) -> Vec<ScheduledEvent> {
    events
        .iter()
        .filter(|e| {
            (selected_maps.is_empty() || selected_maps.iter().any(|m| m == &e.map))
                && (selected_types.is_empty() || selected_types.iter().any(|t| t == &e.name))
        })
        .cloned()
        .collect()
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --no-default-features --features desktop events_view::tests`
Expected: `test result: ok.` with all events_view tests passing (the original 6 + 7 new = 13).

- [ ] **Step 6: Commit**

```bash
git add src/components/events_view.rs
git commit -m "feat(events): add distinct_maps/distinct_types/filter_events helpers with tests"
```

---

## Task 3: `EventFilters` chip component + CSS

**Files:**
- Create: `assets/styling/event_filters.css`
- Create: `src/components/event_filters.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create the stylesheet**

Create `assets/styling/event_filters.css`:

```css
.event-filters {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding-bottom: 0.5rem;
}

.event-filters__group {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.event-filters__label {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--color-text-secondary);
}

.event-filters__chips {
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.event-filters__chip {
  padding: 0.3rem 0.7rem;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  color: var(--color-text-secondary);
  background-color: var(--color-bg-secondary);
  border: 0.0625rem solid var(--color-border);
  border-radius: 1rem;
  white-space: nowrap;
}

.event-filters__chip:hover {
  color: var(--color-text-primary);
}

.event-filters__chip--active {
  color: var(--color-bg-primary);
  background-color: var(--color-accent);
  border-color: var(--color-accent);
}
```

- [ ] **Step 2: Create the component**

Create `src/components/event_filters.rs`:

```rust
use dioxus::prelude::*;

const EVENT_FILTERS_CSS: Asset = asset!("/assets/styling/event_filters.css");

/// Multi-select chip filters for the Events page: a "Maps" group and an
/// "Event Types" group. Stateless — the parent owns the selected lists and
/// toggles a value via the callbacks. Within a group selection is OR; the
/// parent ANDs the two groups together.
#[component]
pub fn EventFilters(
    maps: Vec<String>,
    types: Vec<String>,
    selected_maps: Vec<String>,
    selected_types: Vec<String>,
    on_toggle_map: EventHandler<String>,
    on_toggle_type: EventHandler<String>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: EVENT_FILTERS_CSS }
        div { class: "event-filters",
            if !maps.is_empty() {
                div { class: "event-filters__group",
                    span { class: "event-filters__label", "Maps" }
                    div { class: "event-filters__chips",
                        for map in maps.iter() {
                            {
                                let value = map.clone();
                                let active = selected_maps.iter().any(|m| m == map);
                                let class = if active {
                                    "event-filters__chip event-filters__chip--active"
                                } else {
                                    "event-filters__chip"
                                };
                                rsx! {
                                    button {
                                        key: "{map}",
                                        class: "{class}",
                                        onclick: move |_| on_toggle_map.call(value.clone()),
                                        "{map}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !types.is_empty() {
                div { class: "event-filters__group",
                    span { class: "event-filters__label", "Event Types" }
                    div { class: "event-filters__chips",
                        for ty in types.iter() {
                            {
                                let value = ty.clone();
                                let active = selected_types.iter().any(|t| t == ty);
                                let class = if active {
                                    "event-filters__chip event-filters__chip--active"
                                } else {
                                    "event-filters__chip"
                                };
                                rsx! {
                                    button {
                                        key: "{ty}",
                                        class: "{class}",
                                        onclick: move |_| on_toggle_type.call(value.clone()),
                                        "{ty}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 3: Register the component**

In `src/components/mod.rs`, add (next to the other `mod`/`pub use` pairs):

```rust
mod event_filters;
pub use event_filters::EventFilters;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished`, no errors (an `unused: EventFilters` warning is fine; it is wired in Task 4).

- [ ] **Step 5: Commit**

```bash
git add assets/styling/event_filters.css src/components/event_filters.rs src/components/mod.rs
git commit -m "feat(events): add EventFilters multi-select chip component"
```

---

## Task 4: Wire filters into `EventsView`

**Files:**
- Modify: `src/components/events_view.rs`

- [ ] **Step 1: Import `EventFilters`**

In `src/components/events_view.rs`, change the `super` import line:

```rust
use super::{EventCard, Spinner};
```

to:

```rust
use super::{EventCard, EventFilters, Spinner};
```

- [ ] **Step 2: Add selection signals and the filter pipeline, and render `EventFilters`**

In `src/components/events_view.rs`, replace the body of `EventsView` from the two signal declarations through the end of the `rsx! { ... }` with the version below. (Keep the `use_future` clock loop and the `use_resource` exactly as they are; only the signals at the top and the lines from `let snapshot = ...` onward change.)

Replace:

```rust
    let mut now = use_signal(now_ms);
    let mut refresh = use_signal(|| 0u32);
```

with:

```rust
    let mut now = use_signal(now_ms);
    let mut refresh = use_signal(|| 0u32);
    let mut selected_maps = use_signal(Vec::<String>::new);
    let mut selected_types = use_signal(Vec::<String>::new);
```

Then replace everything from `let snapshot = events_res.read().clone();` through the closing of the `rsx! { ... }` block with:

```rust
    let snapshot = events_res.read().clone();
    let loading = snapshot.is_none();
    let all = snapshot.unwrap_or_default();
    let now_val = now();

    let maps = distinct_maps(&all);
    let types = distinct_types(&all);
    let sel_maps = selected_maps();
    let sel_types = selected_types();
    let filtered = filter_events(&all, &sel_maps, &sel_types);
    let visible = partition_events(&filtered, now_val);
    let render_keys = event_render_keys(&visible);
    let has_active_filters = !sel_maps.is_empty() || !sel_types.is_empty();

    rsx! {
        document::Link { rel: "stylesheet", href: EVENTS_VIEW_CSS }
        div { class: "events-view",
            if loading {
                Spinner { size: "2.5rem".to_string(), label: "Loading events...".to_string() }
            } else {
                if !all.is_empty() {
                    EventFilters {
                        maps: maps.clone(),
                        types: types.clone(),
                        selected_maps: sel_maps.clone(),
                        selected_types: sel_types.clone(),
                        on_toggle_map: move |m: String| {
                            let mut cur = selected_maps();
                            if let Some(pos) = cur.iter().position(|x| x == &m) {
                                cur.remove(pos);
                            } else {
                                cur.push(m);
                            }
                            selected_maps.set(cur);
                        },
                        on_toggle_type: move |t: String| {
                            let mut cur = selected_types();
                            if let Some(pos) = cur.iter().position(|x| x == &t) {
                                cur.remove(pos);
                            } else {
                                cur.push(t);
                            }
                            selected_types.set(cur);
                        },
                    }
                }

                if visible.is_empty() {
                    div { class: "events-view__empty",
                        if all.is_empty() {
                            "Failed to load events."
                        } else if has_active_filters {
                            "No events match the selected filters."
                        } else {
                            "No active or upcoming events."
                        }
                    }
                } else {
                    div { class: "events-view__list",
                        for ((event, state), key) in visible.iter().zip(render_keys.iter()) {
                            EventCard {
                                key: "{key}",
                                name: event.name.clone(),
                                map: event.map.clone(),
                                icon_url: event.icon.clone(),
                                state: *state,
                                now: now_val,
                                start_time: event.start_time,
                                end_time: event.end_time,
                            }
                        }
                    }
                }
            }
        }
    }
```

- [ ] **Step 3: Verify it compiles and all tests pass**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished`, no errors.
Run: `cargo test --no-default-features --features desktop`
Expected: all tests pass (13 in `events_view`, 3 each in `arcs_view` and `event_card` — 19 total).

- [ ] **Step 4: Commit**

```bash
git add src/components/events_view.rs
git commit -m "feat(events): wire map + event-type chip filters into EventsView"
```

---

## Final Verification

- [ ] Full suite: `cargo test --no-default-features --features desktop` — all pass.
- [ ] `cargo check --no-default-features --features desktop` — no errors.
- [ ] Manual (user, on Android via `dx serve --platform android`):
  - All event cards are the same height.
  - Long event names and map names stay on one line; the "Ends in …" / "Starts in …" timer wraps/gives way instead.
  - A "Maps" chip group and an "Event Types" chip group appear above the list.
  - Tapping chips filters the list: multiple chips in one group are OR'd; the two groups are AND'd; selected chips are highlighted; deselecting all restores the full list.
  - With filters active and no matches, the list shows "No events match the selected filters."

## Notes / Decisions (from the spec)

- Event type == event `name`; no API/`arc_api_rs` change.
- Filters are tappable multi-select chips (focused `EventFilters`), not the items `FilterChips`.
- Timer wraps (not font-scaled).
- Distinct chip values derive from all fetched events (stable chip set).
- Pipeline order: fetch → `filter_events` → `partition_events` → `event_render_keys` → render.
