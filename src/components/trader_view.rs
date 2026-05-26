use dioxus::prelude::*;

use super::{CacheDiagnostic, Dropdown, Spinner, TraderItemCard};
use crate::services::source::{CacheSource, CacheState};
use crate::services::traders::{get_trader_items, get_trader_names, trader_items_cache_state, trader_names_cache_state};

const TRADER_VIEW_CSS: Asset = asset!("/assets/styling/trader_view.css");

/// Hardcoded fallback trader names, mirroring the service layer fallback.
/// Used if the async resource fails to resolve.
const FALLBACK_TRADERS: &[&str] = &["Apollo", "Celeste", "Ermal", "Lance", "Shani", "Tian Wen"];

/// Returns the fallback trader names as owned Strings.
fn fallback_options() -> Vec<(String, String)> {
    FALLBACK_TRADERS
        .iter()
        .map(|name| (name.to_string(), name.to_string()))
        .collect()
}

/// The TraderView component fills the content area of the Traders page.
///
/// It fetches trader names from the API (with moka caching and hardcoded fallback),
/// then displays a centered dropdown at the top for selecting a trader.
/// Below the dropdown, a scrollable list of the selected trader's items is shown.
/// If the fetch fails or panics, it gracefully falls back to hardcoded names.
///
/// A debug banner is displayed beneath the dropdown showing data source information
/// to help diagnose API connectivity issues.
#[component]
pub fn TraderView() -> Element {
    // Initialize to the first fallback trader so the resource immediately starts
    // a real async fetch instead of short-circuiting with an empty Vec.
    let mut selected_trader = use_signal(|| FALLBACK_TRADERS[0].to_string());

    // Explicit loading signal — we manage this ourselves inside the resource
    // closure so it is always accurate, regardless of how Dioxus handles stale
    // resource values across re-runs.
    let mut is_loading = use_signal(|| true);

    // Cache source signals for surfacing data source details on the page.
    // Defaults are neutral; the badges are only rendered once loading completes.
    let mut names_source = use_signal(|| CacheSource::Api);
    let mut names_error: Signal<Option<String>> = use_signal(|| None);
    let mut items_source = use_signal(|| CacheSource::Api);
    let mut items_count = use_signal(|| 0usize);
    let mut items_error: Signal<Option<String>> = use_signal(|| None);
    let mut names_state: Signal<Option<CacheState>> = use_signal(|| None);
    let mut items_state: Signal<Option<CacheState>> = use_signal(|| None);

    let trader_names = use_resource(move || async move {
        if cfg!(debug_assertions) {
            names_state.set(Some(trader_names_cache_state()));
        }
        let result = get_trader_names().await;

        names_source.set(result.source);
        names_error.set(result.error.clone());

        result
    });

    let options = match &*trader_names.read() {
        Some(result) if !result.names.is_empty() => result
            .names
            .iter()
            .map(|name| (name.clone(), name.clone()))
            .collect(),
        _ => fallback_options(),
    };

    // Fetch items for the currently selected trader.
    // Reading `selected_trader` signal INSIDE the closure ensures Dioxus
    // tracks the dependency and re-runs the resource when the selection changes.
    let trader_items = use_resource(move || async move {
        let trader_name = selected_trader();

        // Mark loading before the fetch begins.
        is_loading.set(true);

        let result = if trader_name.is_empty() {
            is_loading.set(false);
            return Vec::new();
        } else {
            if cfg!(debug_assertions) {
                items_state.set(Some(trader_items_cache_state(&trader_name)));
            }
            get_trader_items(&trader_name).await
        };

        items_source.set(result.source);
        items_count.set(result.count);
        items_error.set(result.error.clone());

        // Mark loading complete after the fetch resolves (success or failure).
        is_loading.set(false);

        result.items
    });

    let loading = is_loading();
    let items = trader_items.read().clone().unwrap_or_default();

    rsx! {
        document::Link { rel: "stylesheet", href: TRADER_VIEW_CSS }

        div {
            class: "trader-view",

            div {
                class: "trader-view__selector",
                Dropdown {
                    label: String::new(),
                    selected: selected_trader(),
                    options: options,
                    on_change: move |value: String| {
                        selected_trader.set(value);
                    },
                }
            }

            if !loading && cfg!(debug_assertions) {
                div {
                    class: "trader-view__badge",
                    if let Some(state) = names_state() {
                        CacheDiagnostic {
                            source: names_source(),
                            label: Some("Names".to_string()),
                            error: names_error(),
                            state,
                        }
                    }
                    if let Some(state) = items_state() {
                        CacheDiagnostic {
                            source: items_source(),
                            count: Some(items_count()),
                            label: Some("Items".to_string()),
                            error: items_error(),
                            state,
                        }
                    }
                }
            }

            div {
                class: "trader-view__items",

                if loading {
                    Spinner {
                        size: "2.5rem".to_string(),
                        label: "Loading items...".to_string(),
                    }
                } else if items.is_empty() {
                    div {
                        class: "trader-view__items-empty",
                        "No items available for this trader."
                    }
                } else {
                    for item in items.iter() {
                        TraderItemCard {
                            key: "{item.id}",
                            name: item.name.clone(),
                            icon_url: item.icon.0.to_string(),
                            rarity: item.rarity.clone(),
                            item_type: item.item_type.clone(),
                            value: item.value,
                            trader_price: item.trader_price,
                        }
                    }
                }
            }
        }
    }
}
