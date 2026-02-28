use dioxus::prelude::*;

use crate::components::PageLayout;

/// The Map page component that will be rendered when the current route is `[Route::Map]`
#[component]
pub fn Map() -> Element {
    rsx! {
        PageLayout {
            title: "Map",
        }
    }
}
