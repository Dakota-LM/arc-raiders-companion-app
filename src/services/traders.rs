use crate::services::db;
use crate::services::httpclientbuilder::HTTP_CLIENT;
use arc_api_rs::models::traders::{TraderItem, TradersResponse};
use arc_api_rs::MetaForgeClient;
use moka::sync::Cache;
use redb::TableDefinition;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::LazyLock;
use std::time::Duration;

/// Hardcoded fallback trader names in case the API is unavailable.
const FALLBACK_TRADER_NAMES: &[&str] = &["Apollo", "Celeste", "Ermal", "Lance", "Shani", "Tian Wen"];

/// Cache TTL for trader data (15 minutes).
const CACHE_TTL_SECS: u64 = 900;

/// Cache key used for storing trader names.
const TRADER_NAMES_KEY: &str = "trader_names";

/// Cache key prefix for individual trader inventories.
const TRADER_ITEMS_PREFIX: &str = "trader_items_";

const TRADER_NAMES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("trader_names");
const TRADER_ITEMS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("trader_items");

/// Describes where a piece of data originated from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataSource {
    /// Data was fetched live from the MetaForge API.
    Api,
    /// Data was served from the in-memory moka cache.
    Cache,
    /// Data came from the hardcoded fallback constants (API was unreachable or errored).
    Fallback,
}

impl fmt::Display for DataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataSource::Api => write!(f, "API"),
            DataSource::Cache => write!(f, "Cache"),
            DataSource::Fallback => write!(f, "Fallback"),
        }
    }
}

/// Debug metadata returned alongside trader name queries.
#[derive(Debug, Clone)]
pub struct TraderNamesResult {
    /// The list of trader names.
    pub names: Vec<String>,
    /// Where the names were sourced from.
    pub source: DataSource,
    /// If an error occurred (e.g. API failure before fallback), it is captured here.
    pub error: Option<String>,
}

/// Debug metadata returned alongside trader item queries.
#[derive(Debug, Clone)]
pub struct TraderItemsResult {
    /// The list of items for the requested trader.
    pub items: Vec<TraderItem>,
    /// Where the items were sourced from.
    pub source: DataSource,
    /// The number of items returned.
    pub count: usize,
    /// If an error occurred (e.g. API failure), it is captured here.
    pub error: Option<String>,
}

/// Global moka cache for trader name data.
/// Keyed by a string identifier, stores a Vec<String> of trader names.
static TRADERS_NAME_CACHE: LazyLock<Cache<String, Vec<String>>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(16)
        .time_to_live(Duration::from_secs(CACHE_TTL_SECS))
        .build()
});

/// Global moka cache for trader inventory data.
/// Keyed by trader name, stores a Vec<TraderItem> for each trader.
static TRADERS_ITEMS_CACHE: LazyLock<Cache<String, Vec<TraderItem>>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(16)
        .time_to_live(Duration::from_secs(CACHE_TTL_SECS))
        .build()
});

/// Returns the list of trader names along with debug metadata indicating
/// whether they came from the API, cache, or hardcoded fallback.
///
/// This function is guaranteed to never panic. All internal errors
/// (including panics from dependencies) are caught and result in
/// the hardcoded fallback list being returned.
pub async fn get_trader_names() -> TraderNamesResult {
    // L1 Moka
    if let Some(cached) = TRADERS_NAME_CACHE.get(&TRADER_NAMES_KEY.to_string()) {
        return TraderNamesResult { names: cached, source: DataSource::Cache, error: None };
    }
    // L2 fresh redb
    if let Some(names) = db::read_fresh::<Vec<String>>(
        TRADER_NAMES_TABLE,
        TRADER_NAMES_KEY,
        Duration::from_secs(CACHE_TTL_SECS),
    ) {
        TRADERS_NAME_CACHE.insert(TRADER_NAMES_KEY.to_string(), names.clone());
        return TraderNamesResult { names, source: DataSource::Cache, error: None };
    }
    // Source: API
    match fetch_traders_blocking() {
        Ok(resp) => {
            let names = process_and_cache_response(&resp);
            TRADERS_NAME_CACHE.insert(TRADER_NAMES_KEY.to_string(), names.clone());
            db::write(TRADER_NAMES_TABLE, TRADER_NAMES_KEY, &names);
            TraderNamesResult { names, source: DataSource::Api, error: None }
        }
        Err(err) => {
            // Offline fallback: stale redb
            if let Some(names) = db::read_stale::<Vec<String>>(TRADER_NAMES_TABLE, TRADER_NAMES_KEY) {
                TRADERS_NAME_CACHE.insert(TRADER_NAMES_KEY.to_string(), names.clone());
                return TraderNamesResult { names, source: DataSource::Cache, error: Some(err) };
            }
            // Last resort: hardcoded fallback (NOT persisted to redb)
            let names = fallback_trader_names();
            TRADERS_NAME_CACHE.insert(TRADER_NAMES_KEY.to_string(), names.clone());
            TraderNamesResult { names, source: DataSource::Fallback, error: Some(err) }
        }
    }
}

/// Returns the items for a given trader along with debug metadata indicating
/// the source and any errors encountered.
///
/// This function is guaranteed to never panic.
pub async fn get_trader_items(trader_name: &str) -> TraderItemsResult {
    let cache_key = format!("{}{}", TRADER_ITEMS_PREFIX, trader_name);

    // L1 Moka
    if let Some(cached) = TRADERS_ITEMS_CACHE.get(&cache_key) {
        let count = cached.len();
        return TraderItemsResult { items: cached, source: DataSource::Cache, count, error: None };
    }
    // L2 fresh redb
    if let Some(items) = db::read_fresh::<Vec<TraderItem>>(
        TRADER_ITEMS_TABLE,
        trader_name,
        Duration::from_secs(CACHE_TTL_SECS),
    ) {
        let count = items.len();
        TRADERS_ITEMS_CACHE.insert(cache_key.clone(), items.clone());
        return TraderItemsResult { items, source: DataSource::Cache, count, error: None };
    }

    // Cache miss — fetch all traders (populates Moka + redb for every trader)
    let fetch_error = match fetch_traders_blocking() {
        Ok(resp) => {
            process_and_cache_response(&resp);
            None
        }
        Err(err) => Some(err),
    };

    // Re-check Moka after the fetch attempt
    if let Some(items) = TRADERS_ITEMS_CACHE.get(&cache_key) {
        let count = items.len();
        return TraderItemsResult {
            items,
            source: if fetch_error.is_none() {
                DataSource::Api
            } else {
                DataSource::Cache
            },
            count,
            error: fetch_error,
        };
    }

    // API failed and Moka still empty — try stale redb for this trader
    if fetch_error.is_some() {
        if let Some(items) = db::read_stale::<Vec<TraderItem>>(TRADER_ITEMS_TABLE, trader_name) {
            let count = items.len();
            TRADERS_ITEMS_CACHE.insert(cache_key, items.clone());
            return TraderItemsResult { items, source: DataSource::Cache, count, error: fetch_error };
        }
    }

    // Nothing anywhere
    TraderItemsResult {
        items: Vec::new(),
        source: DataSource::Fallback,
        count: 0,
        error: fetch_error.or_else(|| Some(format!("No cached items found for trader '{}'", trader_name))),
    }
}

/// Runs the entire MetaForge API call on a **separate OS thread** with its own
/// tokio runtime, wrapped in `catch_unwind`. This isolates the Dioxus executor
/// from any panics originating in reqwest, TLS, tokio, or the API client.
///
/// Returns `Ok(TradersResponse)` on success, or `Err(String)` if anything
/// (including a panic) goes wrong.
fn fetch_traders_blocking() -> Result<TradersResponse, String> {
    let (tx, rx) = mpsc::channel::<Result<TradersResponse, String>>();

    std::thread::spawn(move || {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to build tokio runtime: {e}"))?;

            rt.block_on(async {
                let http_client = HTTP_CLIENT.clone();

                let client = MetaForgeClient::with_client(http_client);

                client
                    .traders()
                    .await
                    .map_err(|e| format!("API error fetching traders: {e}"))
            })
        }));

        let final_result = match result {
            Ok(inner) => inner,
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic payload".to_string()
                };

                Err(format!("API thread panicked: {msg}"))
            }
        };

        let _ = tx.send(final_result);
    });

    rx.recv()
        .map_err(|e| format!("Failed receiving from API thread: {e}"))?
}

/// Processes a successful `TradersResponse` by caching each trader's inventory
/// and returning the list of trader names.
fn process_and_cache_response(resp: &TradersResponse) -> Vec<String> {
    let traders: [(&str, Option<&[TraderItem]>); 6] = [
        ("Apollo", resp.data.apollo.as_deref()),
        ("Celeste", resp.data.celeste.as_deref()),
        ("Ermal", resp.data.ermal.as_deref()),
        ("Lance", resp.data.lance.as_deref()),
        ("Shani", resp.data.shani.as_deref()),
        ("Tian Wen", resp.data.tian_wen.as_deref()),
    ];

    let mut names: Vec<String> = Vec::new();

    for (name, inventory) in &traders {
        let items = match inventory {
            Some(items) => items.to_vec(),
            None => Vec::new(),
        };

        let cache_key = format!("{}{}", TRADER_ITEMS_PREFIX, name);
        db::write(TRADER_ITEMS_TABLE, name, &items); // write-through to redb (key = bare name)
        TRADERS_ITEMS_CACHE.insert(cache_key, items);
        names.push(name.to_string());
    }

    // If the API returned successfully but yielded no trader names at all,
    // fall back to the hardcoded list.
    if names.is_empty() {
        fallback_trader_names()
    } else {
        names
    }
}

/// Returns the hardcoded list of trader names as owned Strings.
fn fallback_trader_names() -> Vec<String> {
    FALLBACK_TRADER_NAMES
        .iter()
        .map(|name| name.to_string())
        .collect()
}

/// Invalidates all cached trader data, forcing a fresh fetch on the next call.
#[allow(dead_code)]
pub fn invalidate_trader_cache() {
    TRADERS_NAME_CACHE.invalidate(&TRADER_NAMES_KEY.to_string());
    for name in FALLBACK_TRADER_NAMES {
        let cache_key = format!("{}{}", TRADER_ITEMS_PREFIX, name);
        TRADERS_ITEMS_CACHE.invalidate(&cache_key);
    }
}
