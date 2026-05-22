use arc_api_rs::models::ScheduledEvent;
use dioxus::prelude::*;

use crate::components::event_card::EventState;

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
