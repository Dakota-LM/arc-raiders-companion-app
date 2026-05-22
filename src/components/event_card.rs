use dioxus::prelude::*;

const EVENT_CARD_CSS: Asset = asset!("/assets/styling/event_card.css");

/// Whether an event is currently running or has not started yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventState {
    Active,
    Upcoming,
}

/// Format a remaining duration in milliseconds as a compact countdown, dropping
/// unused leading units and the leading zero on the largest shown unit:
/// `2h 05m 30s` / `16m 49s` / `49s`. Negative inputs clamp to zero.
fn format_remaining(ms: i64) -> String {
    let total_secs = ms.max(0) / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {:02}m {:02}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[component]
pub fn EventCard(
    name: String,
    map: String,
    icon_url: String,
    state: EventState,
    now: i64,
    start_time: i64,
    end_time: i64,
) -> Element {
    let remaining_ms = match state {
        EventState::Active => end_time - now,
        EventState::Upcoming => start_time - now,
    };
    let card_class = match state {
        EventState::Active => "event-card",
        EventState::Upcoming => "event-card event-card--upcoming",
    };
    // The countdown is split across the two rows: the label lines up with the
    // event name (top), the time lines up with the map name (bottom).
    let countdown_label = match state {
        EventState::Active => "Ends in:",
        EventState::Upcoming => "Starts in:",
    };
    let countdown_time = format_remaining(remaining_ms);

    rsx! {
        document::Link { rel: "stylesheet", href: EVENT_CARD_CSS }
        div { class: "{card_class}",
            img { class: "event-card__icon", src: "{icon_url}", alt: "{name}" }
            div { class: "event-card__info",
                div { class: "event-card__row",
                    div { class: "event-card__name-wrap",
                        span { class: "event-card__name", "{name}" }
                    }
                    span { class: "event-card__countdown-label", "{countdown_label}" }
                }
                div { class: "event-card__row",
                    span { class: "event-card__map", "{map}" }
                    span { class: "event-card__countdown-time", "{countdown_time}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_unused_leading_units() {
        assert_eq!(format_remaining(0), "0s");
        assert_eq!(format_remaining(59_000), "59s");
        assert_eq!(format_remaining(60_000), "1m 00s");
        assert_eq!(format_remaining(1_009_000), "16m 49s"); // 16m 49s, no hours
    }

    #[test]
    fn shows_hours_minutes_seconds_when_hours_present() {
        assert_eq!(format_remaining(3_600_000), "1h 00m 00s");
        assert_eq!(format_remaining(4_980_000), "1h 23m 00s");
        assert_eq!(format_remaining(36_000_000), "10h 00m 00s");
    }

    #[test]
    fn clamps_negative_to_zero() {
        assert_eq!(format_remaining(-5_000), "0s");
    }
}
