use dioxus::prelude::*;

use crate::components::{ArcsView, PageLayout};

/// The Arcs page component that will be rendered when the current route is `[Route::Arcs]`
#[component]
pub fn Arcs() -> Element {
    rsx! {
        PageLayout {
            title: "Arcs",
            ArcsView {}
        }
    }
}
