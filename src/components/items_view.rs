use std::collections::BTreeSet;

use arc_api_rs::models::Item;
use dioxus::prelude::*;

use super::{FilterChips, ItemCard, Spinner};
use crate::components::filter_chips::{build_filter_options, ActiveFilter};
use crate::components::item_card::extract_stats;
use crate::services::items::get_all_items;

const ITEMS_VIEW_CSS: Asset = asset!("/assets/styling/items_view.css");

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
    let mut expanded_id: Signal<Option<String>> = use_signal(|| None);
    let mut is_loading = use_signal(|| true);
    let mut debug_info = use_signal(|| String::from("Fetching items..."));

    let all_items = use_resource(move || async move {
        is_loading.set(true);
        debug_info.set("Fetching items...".to_string());

        let result = get_all_items().await;

        let mut debug = format!("Source: {} | Count: {}", result.source, result.count);
        if let Some(ref err) = result.error {
            debug.push_str(&format!(" | Error: {}", err));
        }
        debug_info.set(debug);
        is_loading.set(false);

        result.items
    });

    let loading = is_loading();
    let items = all_items.read().clone().unwrap_or_default();

    // Extract filter options from the full dataset
    let (types, rarities, workbenches, slots) = extract_filter_values(&items);
    let filter_options = build_filter_options(&types, &rarities, &workbenches, &slots);

    let sort_options = vec![
        ("name_asc".to_string(), "Name (A-Z)".to_string()),
        ("value_desc".to_string(), "Value (High-Low)".to_string()),
        ("rarity_desc".to_string(), "Rarity (High-Low)".to_string()),
    ];

    // Apply filters and sorting
    let current_filters = active_filters();
    let current_search = search_text();
    let current_sort = sort_by();

    let mut filtered = apply_filters(&items, &current_filters, &current_search);
    sort_items(&mut filtered, &current_sort);
    let filtered_count = filtered.len();

    // Debug banner class
    let debug_text = debug_info();
    let banner_class = if debug_text.contains("Source: API") {
        "items-debug-banner items-debug-banner--api"
    } else if debug_text.contains("Source: Cache") {
        "items-debug-banner items-debug-banner--cache"
    } else {
        "items-debug-banner items-debug-banner--error"
    };

    let current_expanded = expanded_id();

    rsx! {
        document::Link { rel: "stylesheet", href: ITEMS_VIEW_CSS }

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

            // Debug banner
            div {
                class: "items-debug",
                div {
                    class: "{banner_class}",
                    "{debug_text}"
                }
            }

            // Item count
            if !loading && !items.is_empty() {
                div {
                    class: "items-view__count",
                    "Showing {filtered_count} of {items.len()} items"
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
                    for item in filtered.iter() {
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
                }
            }
        }
    }
}
