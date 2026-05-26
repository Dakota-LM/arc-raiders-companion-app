use std::collections::BTreeSet;

use arc_api_rs::models::Item;
use dioxus::prelude::*;

use super::{CacheDiagnostic, FilterChips, ItemCard, Spinner};
use crate::components::filter_chips::{build_filter_options, ActiveFilter};
use crate::components::item_card::extract_stats;
use crate::services::items::{get_all_items, items_cache_state};
use crate::services::source::{CacheSource, CacheState};

const ITEMS_VIEW_CSS: Asset = asset!("/assets/styling/items_view.css");
const ITEM_CARD_CSS: Asset = asset!("/assets/styling/item_card.css");

/// Rarity sort order: higher number = more rare.
fn rarity_rank(rarity: &str) -> u8 {
    match rarity.to_lowercase().as_str() {
        "common" => 1,
        "uncommon" => 2,
        "rare" => 3,
        "epic" => 4,
        "legendary" => 5,
        _ => 0,
    }
}

/// Number of items revealed per scroll batch.
const ITEMS_BATCH_SIZE: usize = 50;

/// Next reveal count after the user scrolls to the sentinel, never exceeding
/// the total number of matching items.
fn next_visible_count(current: usize, total: usize, step: usize) -> usize {
    (current + step).min(total)
}

/// Filters items based on active filters and search text.
/// Within a category, values are OR'd. Across categories, they are AND'd.
fn apply_filters(items: &[Item], filters: &[ActiveFilter], search: &str) -> Vec<Item> {
    let search_lower = search.to_lowercase();

    // Pre-compute filter groups once, not per item
    let type_filters: Vec<&str> = filters
        .iter()
        .filter(|f| f.category == "type")
        .map(|f| f.value.as_str())
        .collect();
    let rarity_filters: Vec<&str> = filters
        .iter()
        .filter(|f| f.category == "rarity")
        .map(|f| f.value.as_str())
        .collect();
    let workbench_filters: Vec<&str> = filters
        .iter()
        .filter(|f| f.category == "workbench")
        .map(|f| f.value.as_str())
        .collect();
    let slot_filters: Vec<&str> = filters
        .iter()
        .filter(|f| f.category == "slot")
        .map(|f| f.value.as_str())
        .collect();

    items
        .iter()
        .filter(|item| {
            // Search filter
            if !search_lower.is_empty() && !item.name.to_lowercase().contains(&search_lower) {
                return false;
            }

            // OR within category, AND across categories
            if !type_filters.is_empty() && !type_filters.contains(&item.item_type.as_str()) {
                return false;
            }
            if !rarity_filters.is_empty() && !rarity_filters.contains(&item.rarity.as_str()) {
                return false;
            }
            if !workbench_filters.is_empty() {
                match &item.workbench {
                    Some(wb) if workbench_filters.contains(&wb.as_str()) => {}
                    _ => return false,
                }
            }
            if !slot_filters.is_empty() {
                let has_match = item
                    .loadout_slots
                    .iter()
                    .any(|s| slot_filters.contains(&s.as_str()));
                if !has_match {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect()
}

/// Sorts items in place based on the selected sort key.
fn sort_items(items: &mut [Item], sort_by: &str) {
    match sort_by {
        "value_desc" => items.sort_by(|a, b| b.value.cmp(&a.value)),
        "rarity_desc" => items.sort_by(|a, b| {
            rarity_rank(&b.rarity)
                .cmp(&rarity_rank(&a.rarity))
                .then_with(|| b.value.cmp(&a.value))
        }),
        // "name_asc" and default
        _ => items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
    }
}

/// Extracts unique sorted values from items for each filter category.
fn extract_filter_values(items: &[Item]) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut types = BTreeSet::new();
    let mut rarities = BTreeSet::new();
    let mut workbenches = BTreeSet::new();
    let mut slots = BTreeSet::new();

    for item in items {
        types.insert(item.item_type.clone());
        rarities.insert(item.rarity.clone());
        if let Some(ref wb) = item.workbench {
            workbenches.insert(wb.clone());
        }
        for slot in &item.loadout_slots {
            slots.insert(slot.clone());
        }
    }

    (
        types.into_iter().collect(),
        rarities.into_iter().collect(),
        workbenches.into_iter().collect(),
        slots.into_iter().collect(),
    )
}

#[component]
pub fn ItemsView() -> Element {
    let mut active_filters: Signal<Vec<ActiveFilter>> = use_signal(Vec::new);
    let mut search_text = use_signal(String::new);
    let mut sort_by = use_signal(|| "name_asc".to_string());
    let mut viewing_cosmetics = use_signal(|| false);
    let mut expanded_id: Signal<Option<String>> = use_signal(|| None);
    let mut is_loading = use_signal(|| true);
    let mut data_source = use_signal(|| CacheSource::Api);
    let mut data_count = use_signal(|| 0usize);
    let mut data_error: Signal<Option<String>> = use_signal(|| None);
    let mut cache_state: Signal<Option<CacheState>> = use_signal(|| None);
    let mut visible_count = use_signal(|| ITEMS_BATCH_SIZE);

    let all_items = use_resource(move || async move {
        is_loading.set(true);
        if cfg!(debug_assertions) {
            cache_state.set(Some(items_cache_state()));
        }
        let result = get_all_items().await;
        data_source.set(result.source);
        data_count.set(result.count);
        data_error.set(result.error.clone());
        is_loading.set(false);
        result.items
    });

    // Reset paging to the first batch whenever the result set changes, so a
    // new search/filter/sort/toggle never keeps a large stale window mounted.
    use_effect(move || {
        let _filters = active_filters();
        let _search = search_text();
        let _sort = sort_by();
        let _cosmetics = viewing_cosmetics();
        visible_count.set(ITEMS_BATCH_SIZE);
    });

    let loading = is_loading();
    let all = all_items.read().clone().unwrap_or_default();
    let is_cosmetics = viewing_cosmetics();

    // Split items into game items (have rarity) and cosmetics (no rarity)
    let items: Vec<Item> = if is_cosmetics {
        all.iter().filter(|i| i.rarity.is_empty()).cloned().collect()
    } else {
        all.iter().filter(|i| !i.rarity.is_empty()).cloned().collect()
    };

    // Extract filter options only for game items view
    let filter_options = if is_cosmetics {
        Vec::new()
    } else {
        let (types, rarities, workbenches, slots) = extract_filter_values(&items);
        build_filter_options(&types, &rarities, &workbenches, &slots)
    };

    let sort_options = if is_cosmetics {
        vec![
            ("name_asc".to_string(), "Name (A-Z)".to_string()),
        ]
    } else {
        vec![
            ("name_asc".to_string(), "Name (A-Z)".to_string()),
            ("value_desc".to_string(), "Value (High-Low)".to_string()),
            ("rarity_desc".to_string(), "Rarity (High-Low)".to_string()),
        ]
    };

    // Apply filters and sorting
    let current_filters = active_filters();
    let current_search = search_text();
    let current_sort = sort_by();

    let mut filtered = apply_filters(&items, &current_filters, &current_search);
    sort_items(&mut filtered, &current_sort);
    let filtered_count = filtered.len();
    let rendered_count = visible_count().min(filtered_count);

    let current_expanded = expanded_id();

    rsx! {
        document::Link { rel: "stylesheet", href: ITEMS_VIEW_CSS }
        document::Link { rel: "stylesheet", href: ITEM_CARD_CSS }

        // Game Items / Cosmetics toggle
        div {
            class: "items-view__toggle",
            button {
                class: if !is_cosmetics { "items-view__toggle-btn items-view__toggle-btn--active" } else { "items-view__toggle-btn" },
                onclick: move |_| {
                    viewing_cosmetics.set(false);
                    active_filters.set(Vec::new());
                },
                "Materials"
            }
            button {
                class: if is_cosmetics { "items-view__toggle-btn items-view__toggle-btn--active" } else { "items-view__toggle-btn" },
                onclick: move |_| {
                    viewing_cosmetics.set(true);
                    active_filters.set(Vec::new());
                },
                "Cosmetics"
            }
        }
        div {
            class: "items-view",

            FilterChips {
                filters: current_filters.clone(),
                filter_options: filter_options,
                search_text: current_search.clone(),
                sort_value: current_sort.clone(),
                sort_options: sort_options,
                on_add_filter: move |filter: ActiveFilter| {
                    let mut current = active_filters();
                    if !current.contains(&filter) {
                        current.push(filter);
                        active_filters.set(current);
                    }
                },
                on_remove_filter: move |filter: ActiveFilter| {
                    let current = active_filters();
                    let updated: Vec<ActiveFilter> = current
                        .into_iter()
                        .filter(|f| f != &filter)
                        .collect();
                    active_filters.set(updated);
                },
                on_clear_filters: move |_| {
                    active_filters.set(Vec::new());
                },
                on_search_change: move |text: String| {
                    search_text.set(text);
                },
                on_sort_change: move |value: String| {
                    sort_by.set(value);
                },
            }


            // Cache diagnostic (dev builds only)
            if !loading && cfg!(debug_assertions) {
                if let Some(state) = cache_state() {
                    div {
                        class: "items-view__badge",
                        CacheDiagnostic {
                            source: data_source(),
                            count: Some(data_count()),
                            error: data_error(),
                            state,
                        }
                    }
                }
            }

            // Item count
            if !loading && !all.is_empty() {
                div {
                    class: "items-view__count",
                    "Showing {rendered_count} of {filtered_count} items"
                }
            }

            div {
                class: "items-view__list",

                if loading {
                    Spinner {
                        size: "2.5rem".to_string(),
                        label: "Loading items...".to_string(),
                    }
                } else if filtered.is_empty() {
                    div {
                        class: "items-view__empty",
                        if items.is_empty() {
                            "Failed to load items."
                        } else {
                            "No items match your filters."
                        }
                    }
                } else {
                    for item in filtered.iter().take(visible_count()) {
                        ItemCard {
                            key: "{item.id}",
                            id: item.id.clone(),
                            name: item.name.clone(),
                            icon_url: item.icon.0.to_string(),
                            rarity: item.rarity.clone(),
                            item_type: item.item_type.clone(),
                            value: item.value,
                            description: item.description.clone(),
                            flavor_text: item.flavor_text.clone(),
                            stats: extract_stats(&item.stat_block),
                            workbench: item.workbench.clone(),
                            ammo_type: item.ammo_type.clone(),
                            loot_area: item.loot_area.clone(),
                            hide_value: is_cosmetics,
                            is_expanded: current_expanded.as_deref() == Some(item.id.as_str()),
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
                    if visible_count() < filtered_count {
                        div {
                            class: "items-view__sentinel",
                            onvisible: move |evt| {
                                if evt.data().is_intersecting().unwrap_or(false) {
                                    let next = next_visible_count(
                                        visible_count(),
                                        filtered_count,
                                        ITEMS_BATCH_SIZE,
                                    );
                                    visible_count.set(next);
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
    fn next_visible_count_advances_by_step() {
        assert_eq!(next_visible_count(50, 312, 50), 100);
    }

    #[test]
    fn next_visible_count_caps_at_total() {
        assert_eq!(next_visible_count(300, 312, 50), 312);
        assert_eq!(next_visible_count(312, 312, 50), 312);
    }

    #[test]
    fn next_visible_count_handles_small_lists() {
        assert_eq!(next_visible_count(50, 10, 50), 10);
    }
}
