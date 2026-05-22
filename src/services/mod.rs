//! The services module contains API interaction logic and caching layers.
//! Each service encapsulates fetching data from the MetaForge API via arc_api_rs,
//! caching results with moka, and providing fallback data when the API is unavailable.

pub mod httpclientbuilder;
pub mod bots;
pub mod items;
pub mod traders;
