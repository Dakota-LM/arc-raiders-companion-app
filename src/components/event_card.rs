use dioxus::prelude::*;

/// Whether an event is currently running or has not started yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventState {
    Active,
    Upcoming,
}

/// Format a remaining duration in milliseconds as a compact countdown string.
/// >= 1 hour -> "Hh MMm"; otherwise "Mm SSs". Negative inputs clamp to zero.
fn format_remaining(ms: i64) -> String {
    let total_secs = ms.max(0) / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, minutes)
    } else {
        format!("{}m {:02}s", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sub_hour_as_minutes_seconds() {
        assert_eq!(format_remaining(0), "0m 00s");
        assert_eq!(format_remaining(59_000), "0m 59s");
        assert_eq!(format_remaining(60_000), "1m 00s");
    }

    #[test]
    fn formats_hours_as_hours_minutes() {
        assert_eq!(format_remaining(3_600_000), "1h 00m");
        assert_eq!(format_remaining(4_980_000), "1h 23m");
    }

    #[test]
    fn clamps_negative_to_zero() {
        assert_eq!(format_remaining(-5_000), "0m 00s");
    }
}
