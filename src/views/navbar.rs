use crate::state::navbar::navbar_display_mode;
use crate::Route;
use dioxus::prelude::*;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

const ICON_EVENTS: Asset = asset!("/assets/styling/media/icons/events.svg");
const ICON_MAP: Asset = asset!("/assets/styling/media/icons/map.svg");
const ICON_RAIDER: Asset = asset!("/assets/styling/media/icons/raider.svg");
const ICON_MATERIALS: Asset = asset!("/assets/styling/media/icons/materials.svg");
const ICON_ARCS: Asset = asset!("/assets/styling/media/icons/arcs.svg");
const ICON_TRADERS: Asset = asset!("/assets/styling/media/icons/traders.svg");
const ICON_SETTINGS: Asset = asset!("/assets/styling/media/icons/settings.svg");

/// The Navbar component that will be rendered on all pages of our app since every page is under the layout.
///
/// This layout component wraps the UI of all routes in a common navbar. The contents of each route
/// will be rendered under the outlet inside this component.
#[component]
pub fn Navbar() -> Element {
    let mode = navbar_display_mode();
    let show_icons = mode.show_icons();
    let show_text = mode.show_text();

    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }

        // Render the page content first, with bottom padding so it isn't hidden behind the fixed navbar
        div {
            style: "padding-bottom: 8vw;",
            Outlet::<Route> {}
        }

        div {
            id: "navbar",
            Link {
                to: Route::Events {},
                class: "navbar__link",
                if show_icons {
                    img { src: ICON_EVENTS, class: "navbar__icon", alt: "Events" }
                }
                if show_text {
                    span { class: "navbar__text", "Events" }
                }
            }
            Link {
                to: Route::Map {},
                class: "navbar__link",
                if show_icons {
                    img { src: ICON_MAP, class: "navbar__icon", alt: "Map" }
                }
                if show_text {
                    span { class: "navbar__text", "Map" }
                }
            }
            Link {
                to: Route::Raider {},
                class: "navbar__link",
                if show_icons {
                    img { src: ICON_RAIDER, class: "navbar__icon", alt: "Raider" }
                }
                if show_text {
                    span { class: "navbar__text", "Raider" }
                }
            }
            Link {
                to: Route::Materials {},
                class: "navbar__link",
                if show_icons {
                    img { src: ICON_MATERIALS, class: "navbar__icon", alt: "Materials" }
                }
                if show_text {
                    span { class: "navbar__text", "Materials" }
                }
            }
            Link {
                to: Route::Arcs {},
                class: "navbar__link",
                if show_icons {
                    img { src: ICON_ARCS, class: "navbar__icon", alt: "Arcs" }
                }
                if show_text {
                    span { class: "navbar__text", "Arcs" }
                }
            }
            Link {
                to: Route::Traders {},
                class: "navbar__link",
                if show_icons {
                    img { src: ICON_TRADERS, class: "navbar__icon", alt: "Traders" }
                }
                if show_text {
                    span { class: "navbar__text", "Traders" }
                }
            }
            Link {
                to: Route::Settings {},
                class: "navbar__link",
                if show_icons {
                    img { src: ICON_SETTINGS, class: "navbar__icon", alt: "Settings" }
                }
                if show_text {
                    span { class: "navbar__text", "Settings" }
                }
            }
        }
    }
}
