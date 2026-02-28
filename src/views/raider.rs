use dioxus::prelude::*;

use crate::components::PageLayout;

/// The Raider page component that will be rendered when the current route is `[Route::Raider]`
#[component]
pub fn Raider() -> Element {
    rsx! {
        PageLayout {
            title: "Raider",
        }
    }
}
