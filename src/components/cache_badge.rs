use dioxus::prelude::*;

use crate::services::source::CacheSource;

const CACHE_BADGE_CSS: Asset = asset!("/assets/styling/cache_badge.css");

/// A small colored chip showing which cache tier served some data.
///
/// # Props
/// - `source`: the tier (API / Memory / Disk / Fallback) — drives the color and text.
/// - `count`: optional item count, appended as `· N`.
/// - `label`: optional leading label, e.g. `"Items"` → `"Items: Memory · 18"`.
/// - `error`: optional error message, appended after the source.
#[component]
pub fn CacheBadge(
    source: CacheSource,
    #[props(default)] count: Option<usize>,
    #[props(default)] label: Option<String>,
    #[props(default)] error: Option<String>,
) -> Element {
    let class = format!("cache-badge cache-badge--{}", source.css_class());

    let mut text = String::new();
    if let Some(label) = &label {
        text.push_str(label);
        text.push_str(": ");
    }
    text.push_str(source.label());
    if let Some(count) = count {
        text.push_str(&format!(" · {count}"));
    }
    if let Some(error) = &error {
        text.push_str(&format!(" · {error}"));
    }

    rsx! {
        document::Link { rel: "stylesheet", href: CACHE_BADGE_CSS }
        div { class: "{class}", "{text}" }
    }
}
