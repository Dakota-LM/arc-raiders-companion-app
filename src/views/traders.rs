use dioxus::prelude::*;

use crate::components::{PageLayout, TraderView};

/// The Traders page component that will be rendered when the current route is `[Route::Traders]`
#[component]
pub fn Traders() -> Element {
    rsx! {
        PageLayout {
            title: "Traders",
            TraderView {}
        }
    }
}
