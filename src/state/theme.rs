use dioxus::prelude::*;

/// Global signal tracking whether dark mode is enabled.
/// Defaults to `true` since the app's base theme is dark.
pub static DARK_MODE: GlobalSignal<bool> = Signal::global(|| true);

/// Returns the current dark mode state.
pub fn dark_mode() -> bool {
    (DARK_MODE)()
}
