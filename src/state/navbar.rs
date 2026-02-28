use dioxus::prelude::*;

/// The display mode for the navbar, controlling whether icons, text, or both are shown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavbarDisplayMode {
    Icons,
    Text,
    Both,
}

impl NavbarDisplayMode {
    /// Returns all available display modes for use in dropdowns and selectors.
    pub fn all() -> &'static [NavbarDisplayMode] {
        &[
            NavbarDisplayMode::Icons,
            NavbarDisplayMode::Text,
            NavbarDisplayMode::Both,
        ]
    }

    /// Returns a human-readable label for the display mode.
    pub fn label(&self) -> &'static str {
        match self {
            NavbarDisplayMode::Icons => "Icons",
            NavbarDisplayMode::Text => "Text",
            NavbarDisplayMode::Both => "Both",
        }
    }

    /// Returns whether icons should be visible in this mode.
    pub fn show_icons(&self) -> bool {
        matches!(self, NavbarDisplayMode::Icons | NavbarDisplayMode::Both)
    }

    /// Returns whether text labels should be visible in this mode.
    pub fn show_text(&self) -> bool {
        matches!(self, NavbarDisplayMode::Text | NavbarDisplayMode::Both)
    }
}

impl std::fmt::Display for NavbarDisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Global signal tracking the current navbar display mode.
/// Defaults to `Both` so users see icons and text labels.
pub static NAVBAR_DISPLAY_MODE: GlobalSignal<NavbarDisplayMode> =
    Signal::global(|| NavbarDisplayMode::Both);

/// Returns the current navbar display mode.
pub fn navbar_display_mode() -> NavbarDisplayMode {
    (NAVBAR_DISPLAY_MODE)()
}
