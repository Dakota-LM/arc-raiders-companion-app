use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arc_api_rs::models::ScheduledEvent;
use dioxus::prelude::*;

use super::{EventCard, Spinner};
use crate::components::event_card::EventState;
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

#[component]
pub fn EventsView() -> Element {
    let mut now = use_signal(now_ms);
    let mut refresh = use_signal(|| 0u32);

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
    let visible = partition_events(&all, now_val);

    rsx! {
        document::Link { rel: "stylesheet", href: EVENTS_VIEW_CSS }
        div { class: "events-view",
            if loading {
                Spinner { size: "2.5rem".to_string(), label: "Loading events...".to_string() }
            } else if visible.is_empty() {
                div { class: "events-view__empty",
                    if all.is_empty() { "Failed to load events." } else { "No active or upcoming events." }
                }
            } else {
                div { class: "events-view__list",
                    for (event, state) in visible.iter() {
                        EventCard {
                            key: "{event.name}-{event.start_time}",
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
}
