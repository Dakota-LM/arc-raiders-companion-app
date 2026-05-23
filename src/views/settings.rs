use dioxus::prelude::*;

use crate::components::{Dropdown, PageLayout, Toggle};
use crate::services::bots::invalidate_bots_cache;
use crate::services::events::invalidate_events_cache;
use crate::services::items::invalidate_items_cache;
use crate::services::traders::invalidate_trader_cache;
use crate::state::navbar::{NavbarDisplayMode, NAVBAR_DISPLAY_MODE};
use crate::state::theme::{dark_mode, DARK_MODE};
use std::time::Duration;

const SETTINGS_CSS: Asset = asset!("/assets/styling/settings.css");

/// The Settings page component that will be rendered when the current route is `[Route::Settings]`
#[component]
pub fn Settings() -> Element {
    let is_dark = dark_mode();
    let mut cache_cleared = use_signal(|| false);

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
        document::Link { rel: "stylesheet", href: SETTINGS_CSS }
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

            div { class: "settings-advanced",
                div { class: "settings-advanced__title", "Advanced" }
                button {
                    class: "settings-advanced__clear-btn",
                    onclick: move |_| {
                        invalidate_items_cache();
                        invalidate_bots_cache();
                        invalidate_events_cache();
                        invalidate_trader_cache();
                        cache_cleared.set(true);
                        spawn(async move {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            cache_cleared.set(false);
                        });
                    },
                    "Clear cache"
                }
                if cache_cleared() {
                    div { class: "settings-advanced__status", "Cache cleared" }
                }
            }
        }
    }
}
