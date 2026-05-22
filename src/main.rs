use dioxus::prelude::*;

use views::{Arcs, Events, Map, Materials, Navbar, Raider, Settings, Traders};

/// Define a components module that contains all shared components for our app.
mod components;
/// Define a services module that contains API interaction logic and caching layers.
mod services;
/// Define a state module that contains global application state managed through Dioxus signals.
mod state;
/// Define a views module that contains the UI for all Layouts and Routes for our app.
mod views;

/// The Route enum is used to define the structure of internal routes in our app. All route enums need to derive
/// the [`Routable`] trait, which provides the necessary methods for the router to work.
///
/// Each variant represents a different URL pattern that can be matched by the router. If that pattern is matched,
/// the components for that route will be rendered.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
        #[route("/")]
        Events {},
        #[route("/map")]
        Map {},
        #[route("/raider")]
        Raider {},
        #[route("/materials")]
        Materials {},
        #[route("/arcs")]
        Arcs {},
        #[route("/settings")]
        Settings {},
        #[route("/traders")]
        Traders {},
}

const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const COMMON_CSS: Asset = asset!("/assets/styling/common.css");

fn main() {
    dioxus::launch(App);
}

/// App is the main component of our app. Components are the building blocks of dioxus apps. Each component is a function
/// that takes some props and returns an Element. In this case, App takes no props because it is the root of our app.
#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: COMMON_CSS }

        Router::<Route> {}
    }
}
