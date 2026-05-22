use dioxus::prelude::*;

use crate::components::{ItemsView, PageLayout};

#[component]
pub fn Items() -> Element {
    rsx! {
        PageLayout {
            title: "Items",
            ItemsView {}
        }
    }
}
