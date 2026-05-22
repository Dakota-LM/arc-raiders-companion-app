# Events Filters → Reuse FilterChips Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the bespoke `EventFilters` chip-cloud with the Materials `FilterChips` component (generalized so search/sort are optional), so the Events filter UI is identical to and cohesive with the Materials page.

**Architecture:** Generalize `FilterChips` with `show_search` / `show_sort` flags (Materials unchanged, defaults `true`); add a pure `build_event_filter_options` (TDD); rewire `EventsView` to hold `active_filters: Vec<ActiveFilter>` and render `FilterChips` with search/sort hidden, deriving map/type selections by category and feeding the existing tested `filter_events`. Delete the bespoke `EventFilters` + its CSS.

**Tech Stack:** Rust, Dioxus 0.7.3 (native, `default = ["mobile"]`), plain CSS.

**Conventions (verified):**
- Test command: `cargo test --no-default-features --features desktop <filter>`
- Check command: `cargo check --no-default-features --features desktop`
- Manual run (user, Android): `dx serve --platform android`. Agents must NOT run `dx serve`.
- `FilterChips`, `ActiveFilter`, `build_filter_options`, `parse_filter_selection` live in `src/components/filter_chips.rs`. The `EventsView` filter helpers `distinct_maps`/`distinct_types`/`filter_events` (already tested) stay as-is; selections are derived from `ActiveFilter` by `category` ("map" / "type").

---

## Task 1: Generalize `FilterChips` + add `build_event_filter_options` (TDD)

**Files:**
- Modify: `src/components/filter_chips.rs`

- [ ] **Step 1: Add the failing test module**

`filter_chips.rs` currently has no tests. Add this test module at the END of the file (it references `build_event_filter_options`, which does not exist yet — so it will not compile):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_options_group_maps_then_types_with_headers() {
        let opts = build_event_filter_options(
            &["Dam".to_string(), "Spaceport".to_string()],
            &["Storm".to_string()],
        );
        assert_eq!(
            opts,
            vec![
                ("__header_map".to_string(), "-- Maps --".to_string()),
                ("map:Dam".to_string(), "Dam".to_string()),
                ("map:Spaceport".to_string(), "Spaceport".to_string()),
                ("__header_type".to_string(), "-- Event Types --".to_string()),
                ("type:Storm".to_string(), "Storm".to_string()),
            ]
        );
    }

    #[test]
    fn event_options_skip_empty_groups() {
        assert!(build_event_filter_options(&[], &[]).is_empty());
        assert_eq!(
            build_event_filter_options(&["Dam".to_string()], &[]),
            vec![
                ("__header_map".to_string(), "-- Maps --".to_string()),
                ("map:Dam".to_string(), "Dam".to_string()),
            ]
        );
    }

    #[test]
    fn map_selection_parses_to_active_filter() {
        let f = parse_filter_selection("map:Dam Battlegrounds").unwrap();
        assert_eq!(f.category, "map");
        assert_eq!(f.value, "Dam Battlegrounds");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --no-default-features --features desktop filter_chips::tests`
Expected: FAILS to compile — `cannot find function build_event_filter_options in this scope`.

- [ ] **Step 3: Add the `build_event_filter_options` function**

In `src/components/filter_chips.rs`, add this function after the existing `build_filter_options` function:

```rust
/// Builds the "Add Filter" dropdown options for the Events page: a Maps group
/// then an Event Types group, each with a non-selectable header, encoded as
/// `"map:<value>"` / `"type:<value>"` (parsed by `parse_filter_selection`).
pub fn build_event_filter_options(maps: &[String], types: &[String]) -> Vec<(String, String)> {
    let mut options: Vec<(String, String)> = Vec::new();

    if !maps.is_empty() {
        options.push(("__header_map".to_string(), "-- Maps --".to_string()));
        for m in maps {
            options.push((format!("map:{}", m), m.clone()));
        }
    }

    if !types.is_empty() {
        options.push(("__header_type".to_string(), "-- Event Types --".to_string()));
        for t in types {
            options.push((format!("type:{}", t), t.clone()));
        }
    }

    options
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --no-default-features --features desktop filter_chips::tests`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Generalize the `FilterChips` component signature**

In `src/components/filter_chips.rs`, replace the component signature:

```rust
#[component]
pub fn FilterChips(
    filters: Vec<ActiveFilter>,
    filter_options: Vec<(String, String)>,
    search_text: String,
    sort_value: String,
    sort_options: Vec<(String, String)>,
    on_add_filter: EventHandler<ActiveFilter>,
    on_remove_filter: EventHandler<ActiveFilter>,
    on_clear_filters: EventHandler<()>,
    on_search_change: EventHandler<String>,
    on_sort_change: EventHandler<String>,
) -> Element {
```

with:

```rust
#[component]
pub fn FilterChips(
    filters: Vec<ActiveFilter>,
    filter_options: Vec<(String, String)>,
    #[props(default = true)] show_search: bool,
    #[props(default = true)] show_sort: bool,
    #[props(default)] search_text: String,
    #[props(default)] sort_value: String,
    #[props(default)] sort_options: Vec<(String, String)>,
    on_add_filter: EventHandler<ActiveFilter>,
    on_remove_filter: EventHandler<ActiveFilter>,
    on_clear_filters: EventHandler<()>,
    on_search_change: EventHandler<String>,
    on_sort_change: EventHandler<String>,
) -> Element {
```

- [ ] **Step 6: Make the controls row conditional**

In the same `rsx!`, replace the existing controls block:

```rust
            // Top row: search + sort dropdown
            div {
                class: "filter-chips__controls",

                input {
                    class: "filter-chips__search",
                    r#type: "text",
                    placeholder: "Search items...",
                    value: "{search_text}",
                    oninput: move |evt: Event<FormData>| {
                        on_search_change.call(evt.value());
                    },
                }

                Dropdown {
                    label: String::new(),
                    selected: sort_value.clone(),
                    options: sort_options.clone(),
                    on_change: move |value: String| {
                        on_sort_change.call(value);
                    },
                }
            }
```

with:

```rust
            // Top row: search + sort dropdown (optional)
            if show_search || show_sort {
                div {
                    class: "filter-chips__controls",

                    if show_search {
                        input {
                            class: "filter-chips__search",
                            r#type: "text",
                            placeholder: "Search items...",
                            value: "{search_text}",
                            oninput: move |evt: Event<FormData>| {
                                on_search_change.call(evt.value());
                            },
                        }
                    }

                    if show_sort {
                        Dropdown {
                            label: String::new(),
                            selected: sort_value.clone(),
                            options: sort_options.clone(),
                            on_change: move |value: String| {
                                on_sort_change.call(value);
                            },
                        }
                    }
                }
            }
```

Leave the filter-row (Add Filter dropdown) and the active-chips row unchanged.

- [ ] **Step 7: Verify the suite passes and Materials still compiles**

Run: `cargo test --no-default-features --features desktop filter_chips::tests`
Expected: `test result: ok. 3 passed`.
Run: `cargo check --no-default-features --features desktop`
Expected: `Finished`, 0 errors. (The Materials `ItemsView` call site passes `search_text`/`sort_value`/`sort_options` explicitly and omits the new flags, so it keeps both rows — unchanged behavior.)

- [ ] **Step 8: Commit**

```bash
git add src/components/filter_chips.rs
git commit -m "feat(filters): make FilterChips search/sort optional; add build_event_filter_options"
```

---

## Task 2: Rewire `EventsView` to use `FilterChips`; delete bespoke `EventFilters`

**Files:**
- Modify: `src/components/events_view.rs`
- Modify: `src/components/mod.rs`
- Delete: `src/components/event_filters.rs`
- Delete: `assets/styling/event_filters.css`

- [ ] **Step 1: Update imports in `events_view.rs`**

Replace:

```rust
use super::{EventCard, EventFilters, Spinner};
use crate::components::event_card::EventState;
use crate::services::events::get_event_schedule;
```

with:

```rust
use super::{EventCard, FilterChips, Spinner};
use crate::components::event_card::EventState;
use crate::components::filter_chips::{build_event_filter_options, ActiveFilter};
use crate::services::events::get_event_schedule;
```

- [ ] **Step 2: Swap the selection signals**

Replace:

```rust
    let mut selected_maps = use_signal(Vec::<String>::new);
    let mut selected_types = use_signal(Vec::<String>::new);
```

with:

```rust
    let mut active_filters: Signal<Vec<ActiveFilter>> = use_signal(Vec::new);
```

- [ ] **Step 3: Replace the derivation + rsx tail**

Replace everything from `let snapshot = events_res.read().clone();` through the end of the `rsx! { ... }` block with:

```rust
    let snapshot = events_res.read().clone();
    let loading = snapshot.is_none();
    let all = snapshot.unwrap_or_default();
    let now_val = now();

    let maps = distinct_maps(&all);
    let types = distinct_types(&all);
    let current_filters = active_filters();
    let sel_maps: Vec<String> = current_filters
        .iter()
        .filter(|f| f.category == "map")
        .map(|f| f.value.clone())
        .collect();
    let sel_types: Vec<String> = current_filters
        .iter()
        .filter(|f| f.category == "type")
        .map(|f| f.value.clone())
        .collect();
    let filtered = filter_events(&all, &sel_maps, &sel_types);
    let visible = partition_events(&filtered, now_val);
    let render_keys = event_render_keys(&visible);
    let event_filter_options = build_event_filter_options(&maps, &types);
    let has_active_filters = !current_filters.is_empty();

    rsx! {
        document::Link { rel: "stylesheet", href: EVENTS_VIEW_CSS }
        div { class: "events-view",
            if loading {
                Spinner { size: "2.5rem".to_string(), label: "Loading events...".to_string() }
            } else {
                if !all.is_empty() {
                    FilterChips {
                        filters: current_filters.clone(),
                        filter_options: event_filter_options,
                        show_search: false,
                        show_sort: false,
                        on_add_filter: move |filter: ActiveFilter| {
                            let mut current = active_filters();
                            if !current.contains(&filter) {
                                current.push(filter);
                                active_filters.set(current);
                            }
                        },
                        on_remove_filter: move |filter: ActiveFilter| {
                            let current = active_filters();
                            let updated: Vec<ActiveFilter> =
                                current.into_iter().filter(|f| f != &filter).collect();
                            active_filters.set(updated);
                        },
                        on_clear_filters: move |_| {
                            active_filters.set(Vec::new());
                        },
                        on_search_change: move |_: String| {},
                        on_sort_change: move |_: String| {},
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

(The clock `use_future` loop, the `use_resource`, and all helper functions + the test module remain unchanged.)

- [ ] **Step 4: Remove the `EventFilters` registration**

In `src/components/mod.rs`, delete these two lines:

```rust
mod event_filters;
pub use event_filters::EventFilters;
```

- [ ] **Step 5: Delete the bespoke component and its CSS**

```bash
git rm src/components/event_filters.rs assets/styling/event_filters.css
```

- [ ] **Step 6: Verify it compiles and the full suite passes**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished`, 0 errors (no remaining references to `EventFilters` or `event_filters.css`).
Run: `cargo test --no-default-features --features desktop`
Expected: all tests pass — 22 total (13 `events_view`, 3 `event_card`, 3 `arcs_view`, 3 `filter_chips`).

- [ ] **Step 7: Manual verification (user, on Android)**

`dx serve --platform android`: the Events page now shows the same "Add Filter" dropdown + chips UI as Materials (no search box, no sort dropdown). Picking maps/event types adds removable chips; multiple chips in a category are OR'd, the two categories AND'd; "Clear all" resets; with filters active and no matches the list shows "No events match the selected filters."

- [ ] **Step 8: Commit**

```bash
git add src/components/events_view.rs src/components/mod.rs
git commit -m "feat(events): reuse FilterChips for map + event-type filters; remove bespoke EventFilters"
```

---

## Final Verification

- [ ] `cargo test --no-default-features --features desktop` — all 22 pass.
- [ ] `cargo check --no-default-features --features desktop` — 0 errors.
- [ ] Manual (user): Events filter UI matches Materials; Materials page filters unchanged (search + sort + filter chips still present and working).

## Notes / Decisions

- DRY/cohesion: Events reuses the Materials `FilterChips` + `filter_chips.css`; the only generalization is optional search/sort via `show_search`/`show_sort` (default `true`, so Materials is untouched).
- The tested `distinct_maps` / `distinct_types` / `filter_events` helpers are retained; selections are derived from `ActiveFilter.category` ("map" / "type").
- No search/sort on the Events page.
