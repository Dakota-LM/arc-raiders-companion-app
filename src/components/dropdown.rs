use dioxus::prelude::*;

const DROPDOWN_CSS: Asset = asset!("/assets/styling/dropdown.css");

/// A generic, reusable custom dropdown component that renders options inline
/// rather than using the native `<select>` element (which opens a modal on mobile).
///
/// The menu defaults to opening below the trigger. If there isn't enough space
/// between the trigger and the navbar, it flips to open above instead.
/// Opening and closing are animated with a slide/fade transition.
///
/// # Props
/// - `label`: The text label displayed next to the dropdown.
/// - `selected`: The currently selected value (as a display string).
/// - `options`: A list of `(value, label)` pairs to populate the dropdown.
/// - `on_change`: Callback fired when a new option is selected, providing the value string.
#[component]
pub fn Dropdown(
    label: String,
    selected: String,
    options: Vec<(String, String)>,
    on_change: EventHandler<String>,
) -> Element {
    let mut is_open = use_signal(|| false);
    let mut open_above = use_signal(|| false);

    // Find the display label for the currently selected value
    let selected_label = options
        .iter()
        .find(|(value, _)| *value == selected)
        .map(|(_, display)| display.clone())
        .unwrap_or(selected.clone());

    let option_count = options.len();

    rsx! {
        document::Link { rel: "stylesheet", href: DROPDOWN_CSS }

        div {
            class: "dropdown",
            span {
                class: "dropdown__label",
                "{label}"
            }
            div {
                class: "dropdown__container",
                button {
                    class: "dropdown__trigger",
                    class: if is_open() { "dropdown__trigger--open" },
                    onclick: move |_| {
                        let will_open = !is_open();
                        if will_open {
                            // Measure available space below the trigger to decide direction.
                            // Estimate each option at roughly 2.5rem (40px) of height plus
                            // a small buffer for border/padding.
                            let count = option_count;
                            spawn(async move {
                                let js = r#"
                                    (function() {
                                        let trigger = document.querySelector('.dropdown__trigger--open')
                                            || document.querySelector('.dropdown__trigger');
                                        if (!trigger) return 'below';
                                        let rect = trigger.getBoundingClientRect();
                                        let viewportHeight = window.innerHeight;
                                        let navbarHeight = viewportHeight * 0.08;
                                        let availableBelow = viewportHeight - rect.bottom - navbarHeight - 10;
                                        return availableBelow < MENU_HEIGHT ? 'above' : 'below';
                                    })()
                                    "#
                                    .replace("MENU_HEIGHT", &format!("{}", count * 40 + 20));
                                let result = document::eval(&js).await;
                                if let Ok(direction) = result {
                                    let dir_str = direction.to_string();
                                    open_above.set(dir_str.contains("above"));
                                }
                            });
                        }
                        is_open.set(will_open);
                    },
                    span { "{selected_label}" }
                    span {
                        class: "dropdown__chevron",
                        class: if is_open() { "dropdown__chevron--open" },
                    }
                }
                div {
                    class: "dropdown__menu",
                    class: if is_open() { "dropdown__menu--open" },
                    class: if open_above() { "dropdown__menu--above" },
                    for (value, display) in options.iter() {
                        button {
                            class: "dropdown__option",
                            class: if *value == selected { "dropdown__option--selected" },
                            onclick: {
                                let value = value.clone();
                                move |_| {
                                    on_change.call(value.clone());
                                    is_open.set(false);
                                }
                            },
                            "{display}"
                        }
                    }
                }
            }
        }
    }
}
