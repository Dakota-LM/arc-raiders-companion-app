use dioxus::prelude::*;

use super::CacheBadge;
use crate::services::source::{CacheSource, CacheState};

/// Dev-only diagnostic row: the served-tier `CacheBadge` plus L1 (Moka) and
/// L2 (redb) state pills, showing the pre-load state of both caches for a key.
#[component]
pub fn CacheDiagnostic(
    source: CacheSource,
    state: CacheState,
    #[props(default)] count: Option<usize>,
    #[props(default)] label: Option<String>,
    #[props(default)] error: Option<String>,
) -> Element {
    let l1_class = format!("cache-pill cache-pill--{}", state.l1.css_class());
    let l2_class = format!("cache-pill cache-pill--{}", state.l2.css_class());

    rsx! {
        div {
            class: "cache-diagnostic",
            CacheBadge { source, count, label, error }
            span { class: "{l1_class}", "L1: {state.l1.label()}" }
            span { class: "{l2_class}", "L2: {state.l2.label()}" }
        }
    }
}
