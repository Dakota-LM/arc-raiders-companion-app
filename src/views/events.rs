use dioxus::prelude::*;

use crate::components::PageLayout;

/// The Events page component that will be rendered when the current route is `[Route::Events]`
#[component]
pub fn Events() -> Element {
    rsx! {
        PageLayout {
            title: "Events",
        }
    }
}
