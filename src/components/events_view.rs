use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arc_api_rs::models::ScheduledEvent;
use dioxus::prelude::*;

use super::{EventCard, FilterChips, Spinner};
use crate::components::event_card::EventState;
use crate::components::filter_chips::{build_event_filter_options, ActiveFilter};
use crate::services::events::get_event_schedule;

const EVENTS_VIEW_CSS: Asset = asset!("/assets/styling/events_view.css");

/// Current wall-clock time in epoch milliseconds.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Partition events relative to `now` (epoch ms):
/// - drops expired events (`end_time <= now`)
/// - returns active events first (sorted by `end_time` ascending),
///   then upcoming events (sorted by `start_time` ascending).
fn partition_events(events: &[ScheduledEvent], now: i64) -> Vec<(ScheduledEvent, EventState)> {
    let mut active: Vec<ScheduledEvent> = Vec::new();
    let mut upcoming: Vec<ScheduledEvent> = Vec::new();
    for e in events {
        if e.end_time <= now {
            continue; // expired
        }
        if e.start_time <= now {
            active.push(e.clone());
        } else {
            upcoming.push(e.clone());
        }
    }
    active.sort_by_key(|e| e.end_time);
    upcoming.sort_by_key(|e| e.start_time);
    active
        .into_iter()
        .map(|e| (e, EventState::Active))
        .chain(upcoming.into_iter().map(|e| (e, EventState::Upcoming)))
        .collect()
}

/// Build a unique render key for every visible event row.
///
/// Dioxus's keyed-list diff asserts that sibling keys are unique; a collision
/// panics during render, which on Android poisons an internal lock and aborts
/// the whole app on the next WebView request. ARC events recur across maps at
/// the same `start_time`, so `name`/`map`/`start_time` are not individually
/// unique — any repeats of the same identity are disambiguated with an
/// occurrence counter so the keys are unique even for duplicate rows.
fn event_render_keys(events: &[(ScheduledEvent, EventState)]) -> Vec<String> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    events
        .iter()
        .map(|(event, _)| {
            let base = format!("{}|{}|{}", event.name, event.map, event.start_time);
            let occurrence = counts.entry(base.clone()).or_insert(0);
            let key = format!("{base}#{occurrence}");
            *occurrence += 1;
            key
        })
        .collect()
}

/// Distinct map names across the events, sorted ascending and de-duplicated.
fn distinct_maps(events: &[ScheduledEvent]) -> Vec<String> {
    let mut maps: Vec<String> = events.iter().map(|e| e.map.clone()).collect();
    maps.sort();
    maps.dedup();
    maps
}

/// Distinct event types (the event `name`), sorted ascending and de-duplicated.
fn distinct_types(events: &[ScheduledEvent]) -> Vec<String> {
    let mut types: Vec<String> = events.iter().map(|e| e.name.clone()).collect();
    types.sort();
    types.dedup();
    types
}

/// Keep events whose map is in `selected_maps` AND whose name is in
/// `selected_types`. An empty selection for a group imposes no constraint on
/// that group (OR within a group, AND across groups).
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

#[component]
pub fn EventsView() -> Element {
    let mut now = use_signal(now_ms);
    let mut refresh = use_signal(|| 0u32);
    let mut active_filters: Signal<Vec<ActiveFilter>> = use_signal(Vec::new);

    // Local clock: tick every second; trigger an API refetch every 60 ticks.
    use_future(move || async move {
        let mut tick: u64 = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            now.set(now_ms());
            tick += 1;
            if tick % 60 == 0 {
                let cur = refresh();
                refresh.set(cur.wrapping_add(1));
            }
        }
    });

    let events_res = use_resource(move || async move {
        let _ = refresh(); // subscribe: re-runs the fetch whenever `refresh` changes
        get_event_schedule().await.events
    });

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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(name: &str, start: i64, end: i64) -> ScheduledEvent {
        ScheduledEvent {
            name: name.to_string(),
            map: "Dam Battlegrounds".to_string(),
            icon: String::new(),
            start_time: start,
            end_time: end,
        }
    }

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

    #[test]
    fn drops_expired_events() {
        let now = 1000;
        let out = partition_events(&[ev("past", 0, 500), ev("live", 0, 2000)], now);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0.name, "live");
    }

    #[test]
    fn active_listed_before_upcoming() {
        let now = 1000;
        let out = partition_events(&[ev("soon", 2000, 3000), ev("live", 500, 2000)], now);
        assert_eq!(out[0].0.name, "live");
        assert_eq!(out[0].1, EventState::Active);
        assert_eq!(out[1].0.name, "soon");
        assert_eq!(out[1].1, EventState::Upcoming);
    }

    #[test]
    fn active_sorted_by_end_time() {
        let now = 1000;
        let out = partition_events(&[ev("ends_late", 0, 5000), ev("ends_soon", 0, 2000)], now);
        assert_eq!(out[0].0.name, "ends_soon");
        assert_eq!(out[1].0.name, "ends_late");
    }

    #[test]
    fn upcoming_sorted_by_start_time() {
        let now = 1000;
        let out = partition_events(&[ev("later", 5000, 6000), ev("sooner", 2000, 3000)], now);
        assert_eq!(out[0].0.name, "sooner");
        assert_eq!(out[1].0.name, "later");
    }

    #[test]
    fn boundaries_start_equals_now_is_active_end_equals_now_is_expired() {
        let now = 1000;
        let out = partition_events(&[ev("starts_now", 1000, 2000), ev("ends_now", 0, 1000)], now);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0.name, "starts_now");
        assert_eq!(out[0].1, EventState::Active);
    }

    #[test]
    fn render_keys_are_unique_even_for_events_sharing_identity() {
        // ARC events recur across maps at the same start_time, so name/map/start_time
        // are not individually unique. Dioxus's keyed-list diff asserts unique sibling
        // keys; a collision panics during render and (on Android) aborts the whole app.
        // Render keys MUST be unique even for byte-for-byte duplicate event rows.
        let visible = vec![
            (ev("World Boss", 0, 1000), EventState::Active),
            (ev("World Boss", 0, 1000), EventState::Active),
            (ev("World Boss", 0, 1000), EventState::Active),
        ];
        let keys = event_render_keys(&visible);
        let unique: std::collections::HashSet<_> = keys.iter().collect();
        assert_eq!(keys.len(), 3, "one key per rendered row");
        assert_eq!(unique.len(), keys.len(), "all render keys must be unique");
    }
}
