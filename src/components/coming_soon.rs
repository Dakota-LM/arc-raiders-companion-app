use dioxus::prelude::*;

const DEFAULT_SUBTITLE: &str = "This feature is in development.";

#[component]
pub fn ComingSoon(icon: String, subtitle: Option<String>) -> Element {
    let _ = (icon, subtitle);
    rsx! {}
}

/// Returns the subtitle to display: the provided text, or the default copy when
/// `subtitle` is `None` or blank.
fn resolve_subtitle(subtitle: &Option<String>) -> String {
    subtitle
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SUBTITLE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_uses_default_copy() {
        assert_eq!(resolve_subtitle(&None), "This feature is in development.");
    }

    #[test]
    fn blank_falls_back_to_default() {
        assert_eq!(
            resolve_subtitle(&Some("   ".to_string())),
            "This feature is in development."
        );
    }

    #[test]
    fn custom_subtitle_is_used() {
        assert_eq!(
            resolve_subtitle(&Some("Map tools soon".to_string())),
            "Map tools soon"
        );
    }
}
