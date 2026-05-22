use dioxus::prelude::*;

const EVENT_CARD_CSS: Asset = asset!("/assets/styling/event_card.css");

/// Whether an event is currently running or has not started yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventState {
    Active,
    Upcoming,
}

/// Format a remaining duration in milliseconds as a fixed-width `HHh:MMm:SSs`
/// countdown (e.g. `00h:16m:49s`). Negative inputs clamp to zero. The fixed
/// shape lets it sit in a compact two-line block so event/map names get the room.
fn format_remaining(ms: i64) -> String {
    let total_secs = ms.max(0) / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    format!("{:02}h:{:02}m:{:02}s", hours, minutes, seconds)
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
                div { class: "event-card__name-wrap",
                    span { class: "event-card__name", "{name}" }
                }
                span { class: "event-card__map", "{map}" }
            }
            div { class: "event-card__countdown",
                span { class: "event-card__countdown-label", "{countdown_label}" }
                span { class: "event-card__countdown-time", "{countdown_time}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_as_fixed_hms() {
        assert_eq!(format_remaining(0), "00h:00m:00s");
        assert_eq!(format_remaining(59_000), "00h:00m:59s");
        assert_eq!(format_remaining(60_000), "00h:01m:00s");
    }

    #[test]
    fn formats_hours_with_two_digits() {
        assert_eq!(format_remaining(3_600_000), "01h:00m:00s");
        assert_eq!(format_remaining(4_980_000), "01h:23m:00s");
        assert_eq!(format_remaining(36_000_000), "10h:00m:00s");
    }

    #[test]
    fn clamps_negative_to_zero() {
        assert_eq!(format_remaining(-5_000), "00h:00m:00s");
    }
}
