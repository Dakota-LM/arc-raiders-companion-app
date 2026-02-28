use dioxus::prelude::*;

const TOGGLE_CSS: Asset = asset!("/assets/styling/toggle.css");

/// A generic, reusable toggle switch component.
///
/// # Props
/// - `label`: The text label displayed next to the toggle.
/// - `enabled`: Whether the toggle is currently on.
/// - `on_toggle`: Callback fired when the toggle is clicked, providing the new boolean value.
#[component]
pub fn Toggle(label: String, enabled: bool, on_toggle: EventHandler<bool>) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: TOGGLE_CSS }

        div {
            class: "toggle",
            span {
                class: "toggle__label",
                "{label}"
            }
            button {
                class: "toggle__button",
                class: if enabled { "toggle__button--active" },
                onclick: move |_| {
                    on_toggle.call(!enabled);
                },
                div {
                    class: "toggle__knob",
                }
            }
        }
    }
}
