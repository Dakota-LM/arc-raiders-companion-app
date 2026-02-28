use dioxus::prelude::*;

use crate::components::{Dropdown, PageLayout, Toggle};
use crate::state::navbar::{NavbarDisplayMode, NAVBAR_DISPLAY_MODE};
use crate::state::theme::{dark_mode, DARK_MODE};

/// The Settings page component that will be rendered when the current route is `[Route::Settings]`
#[component]
pub fn Settings() -> Element {
    let is_dark = dark_mode();

    use_effect(move || {
        if dark_mode() {
            document::eval(r#"document.documentElement.classList.remove('light')"#);
        } else {
            document::eval(r#"document.documentElement.classList.add('light')"#);
        }
    });

    let current_navbar_mode = (NAVBAR_DISPLAY_MODE)();

    let navbar_options: Vec<(String, String)> = NavbarDisplayMode::all()
        .iter()
        .map(|mode| (mode.label().to_string(), mode.label().to_string()))
        .collect();

    rsx! {
        PageLayout {
            title: "Settings",
            Toggle {
                label: "Dark Mode",
                enabled: is_dark,
                on_toggle: move |value: bool| {
                    *DARK_MODE.write() = value;
                },
            }
            Dropdown {
                label: "Navbar",
                selected: current_navbar_mode.label().to_string(),
                options: navbar_options,
                on_change: move |value: String| {
                    let new_mode = match value.as_str() {
                        "Icons" => NavbarDisplayMode::Icons,
                        "Text" => NavbarDisplayMode::Text,
                        _ => NavbarDisplayMode::Both,
                    };
                    *NAVBAR_DISPLAY_MODE.write() = new_mode;
                },
            }
        }
    }
}
