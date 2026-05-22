use dioxus::prelude::*;

const EVENT_FILTERS_CSS: Asset = asset!("/assets/styling/event_filters.css");

/// Multi-select chip filters for the Events page: a "Maps" group and an
/// "Event Types" group. Stateless — the parent owns the selected lists and
/// toggles a value via the callbacks. Within a group selection is OR; the
/// parent ANDs the two groups together.
#[component]
pub fn EventFilters(
    maps: Vec<String>,
    types: Vec<String>,
    selected_maps: Vec<String>,
    selected_types: Vec<String>,
    on_toggle_map: EventHandler<String>,
    on_toggle_type: EventHandler<String>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: EVENT_FILTERS_CSS }
        div { class: "event-filters",
            if !maps.is_empty() {
                div { class: "event-filters__group",
                    span { class: "event-filters__label", "Maps" }
                    div { class: "event-filters__chips",
                        for map in maps.iter() {
                            {
                                let value = map.clone();
                                let active = selected_maps.iter().any(|m| m == map);
                                let class = if active {
                                    "event-filters__chip event-filters__chip--active"
                                } else {
                                    "event-filters__chip"
                                };
                                rsx! {
                                    button {
                                        key: "{map}",
                                        class: "{class}",
                                        onclick: move |_| on_toggle_map.call(value.clone()),
                                        "{map}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !types.is_empty() {
                div { class: "event-filters__group",
                    span { class: "event-filters__label", "Event Types" }
                    div { class: "event-filters__chips",
                        for ty in types.iter() {
                            {
                                let value = ty.clone();
                                let active = selected_types.iter().any(|t| t == ty);
                                let class = if active {
                                    "event-filters__chip event-filters__chip--active"
                                } else {
                                    "event-filters__chip"
                                };
                                rsx! {
                                    button {
                                        key: "{ty}",
                                        class: "{class}",
                                        onclick: move |_| on_toggle_type.call(value.clone()),
                                        "{ty}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
