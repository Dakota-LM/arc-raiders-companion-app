use dioxus::prelude::*;

use super::Dropdown;

const FILTER_CHIPS_CSS: Asset = asset!("/assets/styling/filter_chips.css");

/// A single active filter, identified by its category and value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFilter {
    pub category: String,
    pub value: String,
}

/// Builds the options list for the "Add Filter" dropdown.
/// Each option is `(value, display_label)` where value encodes `"category:value"`.
/// Category headers are included as non-selectable entries.
pub fn build_filter_options(
    types: &[String],
    rarities: &[String],
    workbenches: &[String],
    loadout_slots: &[String],
) -> Vec<(String, String)> {
    let mut options: Vec<(String, String)> = Vec::new();

    if !types.is_empty() {
        options.push(("__header_type".to_string(), "-- Type --".to_string()));
        for t in types {
            options.push((format!("type:{}", t), t.clone()));
        }
    }

    if !rarities.is_empty() {
        options.push(("__header_rarity".to_string(), "-- Rarity --".to_string()));
        for r in rarities {
            options.push((format!("rarity:{}", r), r.clone()));
        }
    }

    if !workbenches.is_empty() {
        options.push(("__header_workbench".to_string(), "-- Workbench --".to_string()));
        for w in workbenches {
            options.push((format!("workbench:{}", w), w.clone()));
        }
    }

    if !loadout_slots.is_empty() {
        options.push(("__header_slot".to_string(), "-- Loadout Slot --".to_string()));
        for s in loadout_slots {
            options.push((format!("slot:{}", s), s.clone()));
        }
    }

    options
}

/// Builds the "Add Filter" dropdown options for the Events page: a Maps group
/// then an Event Types group, each with a non-selectable header, encoded as
/// `"map:<value>"` / `"type:<value>"` (parsed by `parse_filter_selection`).
pub fn build_event_filter_options(maps: &[String], types: &[String]) -> Vec<(String, String)> {
    let mut options: Vec<(String, String)> = Vec::new();

    if !maps.is_empty() {
        options.push(("__header_map".to_string(), "-- Maps --".to_string()));
        for m in maps {
            options.push((format!("map:{}", m), m.clone()));
        }
    }

    if !types.is_empty() {
        options.push(("__header_type".to_string(), "-- Event Types --".to_string()));
        for t in types {
            options.push((format!("type:{}", t), t.clone()));
        }
    }

    options
}

/// Parses a dropdown selection value like `"type:Weapon"` into an `ActiveFilter`.
pub fn parse_filter_selection(selection: &str) -> Option<ActiveFilter> {
    let (category, value) = selection.split_once(':')?;
    Some(ActiveFilter {
        category: category.to_string(),
        value: value.to_string(),
    })
}

#[component]
pub fn FilterChips(
    filters: Vec<ActiveFilter>,
    filter_options: Vec<(String, String)>,
    #[props(default = true)] show_search: bool,
    #[props(default = true)] show_sort: bool,
    #[props(default)] search_text: String,
    #[props(default)] sort_value: String,
    #[props(default)] sort_options: Vec<(String, String)>,
    on_add_filter: EventHandler<ActiveFilter>,
    on_remove_filter: EventHandler<ActiveFilter>,
    on_clear_filters: EventHandler<()>,
    on_search_change: EventHandler<String>,
    on_sort_change: EventHandler<String>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: FILTER_CHIPS_CSS }

        div {
            class: "filter-chips",

            // Top row: search + sort dropdown (optional)
            if show_search || show_sort {
                div {
                    class: "filter-chips__controls",

                    if show_search {
                        input {
                            class: "filter-chips__search",
                            r#type: "text",
                            placeholder: "Search items...",
                            value: "{search_text}",
                            oninput: move |evt: Event<FormData>| {
                                on_search_change.call(evt.value());
                            },
                        }
                    }

                    if show_sort {
                        Dropdown {
                            label: String::new(),
                            selected: sort_value.clone(),
                            options: sort_options.clone(),
                            on_change: move |value: String| {
                                on_sort_change.call(value);
                            },
                        }
                    }
                }
            }

            // Filter dropdown row: centred
            if !filter_options.is_empty() {
                div {
                    class: "filter-chips__filter-row",
                    Dropdown {
                        label: String::new(),
                        selected: "Add Filter".to_string(),
                        options: filter_options.clone(),
                        on_change: move |value: String| {
                            if !value.starts_with("__header_") {
                                if let Some(filter) = parse_filter_selection(&value) {
                                    on_add_filter.call(filter);
                                }
                            }
                        },
                    }
                }
            }

            // Active chips row
            if !filters.is_empty() {
                div {
                    class: "filter-chips__active",

                    for filter in filters.iter() {
                        {
                            let filter_clone = filter.clone();
                            rsx! {
                                button {
                                    class: "filter-chips__chip",
                                    onclick: move |_| on_remove_filter.call(filter_clone.clone()),
                                    "{filter.value}"
                                    span { class: "filter-chips__chip-x", "\u{00d7}" }
                                }
                            }
                        }
                    }

                    button {
                        class: "filter-chips__clear",
                        onclick: move |_| on_clear_filters.call(()),
                        "Clear all"
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
    fn event_options_group_maps_then_types_with_headers() {
        let opts = build_event_filter_options(
            &["Dam".to_string(), "Spaceport".to_string()],
            &["Storm".to_string()],
        );
        assert_eq!(
            opts,
            vec![
                ("__header_map".to_string(), "-- Maps --".to_string()),
                ("map:Dam".to_string(), "Dam".to_string()),
                ("map:Spaceport".to_string(), "Spaceport".to_string()),
                ("__header_type".to_string(), "-- Event Types --".to_string()),
                ("type:Storm".to_string(), "Storm".to_string()),
            ]
        );
    }

    #[test]
    fn event_options_skip_empty_groups() {
        assert!(build_event_filter_options(&[], &[]).is_empty());
        assert_eq!(
            build_event_filter_options(&["Dam".to_string()], &[]),
            vec![
                ("__header_map".to_string(), "-- Maps --".to_string()),
                ("map:Dam".to_string(), "Dam".to_string()),
            ]
        );
    }

    #[test]
    fn map_selection_parses_to_active_filter() {
        let f = parse_filter_selection("map:Dam Battlegrounds").unwrap();
        assert_eq!(f.category, "map");
        assert_eq!(f.value, "Dam Battlegrounds");
    }
}
