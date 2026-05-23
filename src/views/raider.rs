use dioxus::prelude::*;

use crate::components::{ComingSoon, PageLayout};

const ICON_RAIDER: Asset = asset!("/assets/styling/media/icons/raider.svg");

/// The Raider page component that will be rendered when the current route is `[Route::Raider]`
#[component]
pub fn Raider() -> Element {
    rsx! {
        PageLayout {
            title: "Raider",
            ComingSoon { icon: ICON_RAIDER.to_string() }
        }
    }
}
