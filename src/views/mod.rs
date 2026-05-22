//! The views module contains the components for all Layouts and Routes for our app. Each layout and route in our [`Route`]
//! enum will render one of these components.
//!
//! The [`Navbar`] component will be rendered on all pages of our app since every page is under the layout. The layout defines
//! a common wrapper around all child routes.

mod events;
pub use events::Events;

mod map;
pub use map::Map;

mod raider;
pub use raider::Raider;

mod materials;
pub use materials::Materials;

mod arcs;
pub use arcs::Arcs;

mod settings;
pub use settings::Settings;

mod traders;
pub use traders::Traders;

mod items;
pub use items::Items;

mod navbar;
pub use navbar::Navbar;
