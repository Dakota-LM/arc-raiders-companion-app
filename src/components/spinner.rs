use dioxus::prelude::*;

const SPINNER_CSS: Asset = asset!("/assets/styling/spinner.css");

/// A reusable animated spinner (progress wheel) component.
///
/// Displays a circular spinning indicator, optionally with a text label beneath it.
/// Uses CSS variables from the app theme for consistent styling in both dark and light modes.
///
/// # Props
/// - `size`: Optional diameter of the spinner in CSS units (default: `"2.5rem"`).
/// - `label`: Optional text to display below the spinner (e.g. `"Loading items..."`).
#[component]
pub fn Spinner(
    #[props(default = "2.5rem".to_string())] size: String,
    #[props(default)] label: Option<String>,
) -> Element {
    let spinner_style = format!("width: {size}; height: {size};");

    rsx! {
        document::Link { rel: "stylesheet", href: SPINNER_CSS }

        div {
            class: "spinner-container",

            div {
                class: "spinner",
                style: "{spinner_style}",
            }

            if let Some(text) = &label {
                span {
                    class: "spinner__label",
                    "{text}"
                }
            }
        }
    }
}
