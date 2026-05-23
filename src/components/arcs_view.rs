use arc_api_rs::models::Bot;
use dioxus::prelude::*;

use super::{ArcCard, CacheDiagnostic, Spinner};
use crate::services::bots::{get_all_bots, bots_cache_state};
use crate::services::source::{CacheSource, CacheState};

const ARCS_VIEW_CSS: Asset = asset!("/assets/styling/arcs_view.css");

/// Returns true if `name` should be kept for the given search `query`.
/// An empty / whitespace-only query matches everything; otherwise matching is
/// a case-insensitive substring test.
fn name_matches(name: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&q)
}

/// Filter bots by name search, then sort alphabetically (A–Z, or Z–A if `sort_desc`).
fn filter_and_sort_bots(bots: &[Bot], search: &str, sort_desc: bool) -> Vec<Bot> {
    let mut out: Vec<Bot> = bots
        .iter()
        .filter(|b| name_matches(&b.name, search))
        .cloned()
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    if sort_desc {
        out.reverse();
    }
    out
}

#[component]
pub fn ArcsView() -> Element {
    let mut search = use_signal(String::new);
    let mut sort_desc = use_signal(|| false);
    let mut expanded_id: Signal<Option<String>> = use_signal(|| None);
    let mut is_loading = use_signal(|| true);
    let mut data_source = use_signal(|| CacheSource::Api);
    let mut data_count = use_signal(|| 0usize);
    let mut data_error: Signal<Option<String>> = use_signal(|| None);
    let mut cache_state: Signal<Option<CacheState>> = use_signal(|| None);

    let bots_res = use_resource(move || async move {
        is_loading.set(true);
        if cfg!(debug_assertions) {
            cache_state.set(Some(bots_cache_state()));
        }
        let result = get_all_bots().await;
        data_source.set(result.source);
        data_count.set(result.count);
        data_error.set(result.error.clone());
        is_loading.set(false);
        result.bots
    });

    let loading = is_loading();
    let all = bots_res.read().clone().unwrap_or_default();
    let search_val = search();
    let desc = sort_desc();
    let filtered = filter_and_sort_bots(&all, &search_val, desc);
    let current_expanded = expanded_id();

    rsx! {
        document::Link { rel: "stylesheet", href: ARCS_VIEW_CSS }
        div { class: "arcs-view",
            div { class: "arcs-view__controls",
                input {
                    class: "arcs-view__search",
                    r#type: "text",
                    placeholder: "Search arcs...",
                    value: "{search_val}",
                    oninput: move |e| search.set(e.value()),
                }
                button {
                    class: "arcs-view__sort-btn",
                    onclick: move |_| {
                        let cur = sort_desc();
                        sort_desc.set(!cur);
                    },
                    if desc { "Z–A" } else { "A–Z" }
                }
            }

            if !loading && cfg!(debug_assertions) {
                if let Some(state) = cache_state() {
                    div { class: "arcs-view__badge",
                        CacheDiagnostic {
                            source: data_source(),
                            count: Some(data_count()),
                            error: data_error(),
                            state,
                        }
                    }
                }
            }

            if loading {
                Spinner { size: "2.5rem".to_string(), label: "Loading arcs...".to_string() }
            } else if filtered.is_empty() {
                div { class: "arcs-view__empty",
                    if all.is_empty() { "Failed to load arcs." } else { "No arcs match your search." }
                }
            } else {
                div { class: "arcs-view__list",
                    for bot in filtered.iter() {
                        ArcCard {
                            key: "{bot.id}",
                            id: bot.id.clone(),
                            name: bot.name.clone(),
                            icon_url: bot.icon.as_ref().map(|u| u.0.to_string()).unwrap_or_default(),
                            image_url: bot.image.as_ref().map(|u| u.0.to_string()),
                            description: bot.description.clone().unwrap_or_default(),
                            is_expanded: current_expanded.as_deref() == Some(bot.id.as_str()),
                            on_toggle: move |id: String| {
                                let current = expanded_id();
                                if current.as_deref() == Some(id.as_str()) {
                                    expanded_id.set(None);
                                } else {
                                    expanded_id.set(Some(id));
                                }
                            },
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

    #[test]
    fn empty_query_matches_everything() {
        assert!(name_matches("Rocketeer", ""));
        assert!(name_matches("Anything", "   "));
    }

    #[test]
    fn matches_case_insensitive_substring() {
        assert!(name_matches("Rocketeer", "rocket"));
        assert!(name_matches("rocketeer", "ROCKET"));
        assert!(name_matches("Tick Bot", "bot"));
    }

    #[test]
    fn non_match_returns_false() {
        assert!(!name_matches("Bombardier", "rocket"));
    }
}
