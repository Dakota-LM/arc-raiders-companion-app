use dioxus::prelude::*;

use super::Dropdown;
use crate::components::filter_chips::ActiveFilter;

// Reuse the Materials chip styling for cohesion, plus a small layout sheet for
// the two dropdowns.
const FILTER_CHIPS_CSS: Asset = asset!("/assets/styling/filter_chips.css");
const EVENT_FILTERS_CSS: Asset = asset!("/assets/styling/event_filters.css");

/// Events-page filters: two dropdowns ("Map", "Event Type") that each add a
/// removable chip below (multi-select; OR within a group, AND across groups).
/// Stateless — the parent owns the `filters` list and toggles it via callbacks.
#[component]
pub fn EventFilters(
    maps: Vec<String>,
    types: Vec<String>,
    filters: Vec<ActiveFilter>,
    on_add: EventHandler<ActiveFilter>,
    on_remove: EventHandler<ActiveFilter>,
    on_clear: EventHandler<()>,
) -> Element {
    let map_options: Vec<(String, String)> = maps.iter().map(|m| (m.clone(), m.clone())).collect();
    let type_options: Vec<(String, String)> =
        types.iter().map(|t| (t.clone(), t.clone())).collect();

    rsx! {
        document::Link { rel: "stylesheet", href: FILTER_CHIPS_CSS }
        document::Link { rel: "stylesheet", href: EVENT_FILTERS_CSS }

        div { class: "event-filters",
            div { class: "event-filters__dropdowns",
                Dropdown {
                    label: String::new(),
                    selected: "Map".to_string(),
                    options: map_options,
                    on_change: move |value: String| {
                        on_add.call(ActiveFilter { category: "map".to_string(), value });
                    },
                }
                Dropdown {
                    label: String::new(),
                    selected: "Event Type".to_string(),
                    options: type_options,
                    on_change: move |value: String| {
                        on_add.call(ActiveFilter { category: "type".to_string(), value });
                    },
                }
            }

            if !filters.is_empty() {
                div { class: "filter-chips__active",
                    for filter in filters.iter() {
                        {
                            let filter_clone = filter.clone();
                            rsx! {
                                button {
                                    key: "{filter.category}:{filter.value}",
                                    class: "filter-chips__chip",
                                    onclick: move |_| on_remove.call(filter_clone.clone()),
                                    "{filter.value}"
                                    span { class: "filter-chips__chip-x", "\u{00d7}" }
                                }
                            }
                        }
                    }

                    button {
                        class: "filter-chips__clear",
                        onclick: move |_| on_clear.call(()),
                        "Clear all"
                    }
                }
            }
        }
    }
}
