use dioxus::prelude::*;

use crate::components::{ComingSoon, PageLayout};

const ICON_MAP: Asset = asset!("/assets/styling/media/icons/map.svg");

/// The Map page component that will be rendered when the current route is `[Route::Map]`
#[component]
pub fn Map() -> Element {
    rsx! {
        PageLayout {
            title: "Map",
            ComingSoon { icon: ICON_MAP.to_string() }
        }
    }
}
