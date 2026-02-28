use dioxus::prelude::*;

use crate::components::PageLayout;

/// The Materials page component that will be rendered when the current route is `[Route::Materials]`
#[component]
pub fn Materials() -> Element {
    rsx! {
        PageLayout {
            title: "Materials",
        }
    }
}
