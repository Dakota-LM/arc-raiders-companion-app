use dioxus::prelude::*;

const COMING_SOON_CSS: Asset = asset!("/assets/styling/coming_soon.css");
const DEFAULT_SUBTITLE: &str = "This feature is in development.";

/// Reusable centered "Coming Soon" notice card for pages that are not yet built.
///
/// # Props
/// - `icon`: asset path for the page's icon (e.g. the Map or Raider icon).
/// - `subtitle`: optional override; `None` or blank uses the default copy.
#[component]
pub fn ComingSoon(icon: String, subtitle: Option<String>) -> Element {
    let subtitle = resolve_subtitle(&subtitle);

    rsx! {
        document::Link { rel: "stylesheet", href: COMING_SOON_CSS }
        div { class: "coming-soon",
            div { class: "coming-soon__card",
                img { class: "coming-soon__icon", src: "{icon}", alt: "Coming soon" }
                h2 { class: "coming-soon__title", "Coming Soon" }
                p { class: "coming-soon__subtitle", "{subtitle}" }
            }
        }
    }
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
