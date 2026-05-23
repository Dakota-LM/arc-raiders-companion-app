use crate::services::db;
use crate::services::httpclientbuilder::HTTP_CLIENT;
use arc_api_rs::endpoints::items::ItemsQuery;
use arc_api_rs::models::Item;
use arc_api_rs::MetaForgeClient;
use moka::sync::Cache;
use redb::TableDefinition;
use crate::services::source::CacheSource;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::LazyLock;
use std::time::Duration;

const CACHE_TTL_SECS: u64 = 900;
const ITEMS_CACHE_KEY: &str = "all_items";
const ITEMS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("items");

#[derive(Debug, Clone)]
pub struct ItemsResult {
    pub items: Vec<Item>,
    pub source: CacheSource,
    pub count: usize,
    pub error: Option<String>,
}

static ITEMS_CACHE: LazyLock<Cache<String, Vec<Item>>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(4)
        .time_to_live(Duration::from_secs(CACHE_TTL_SECS))
        .build()
});

pub async fn get_all_items() -> ItemsResult {
    // Captures which branch the loader took, so we can report the right CacheSource.
    let resolved: RefCell<Option<CacheSource>> = RefCell::new(None);

    // Moka L1 loader: the closure runs only on an L1 miss, and on success its value
    // is inserted into Moka (warming the cache). On Err nothing is cached.
    let outcome = ITEMS_CACHE
        .entry(ITEMS_CACHE_KEY.to_string())
        .or_try_insert_with(|| -> Result<Vec<Item>, String> {
            // L2: fresh redb
            if let Some(items) = db::read_fresh::<Vec<Item>>(
                ITEMS_TABLE,
                ITEMS_CACHE_KEY,
                Duration::from_secs(CACHE_TTL_SECS),
            ) {
                *resolved.borrow_mut() = Some(CacheSource::Disk);
                return Ok(items);
            }
            // Source: API (write-through to redb on success)
            match fetch_items_blocking() {
                Ok(items) => {
                    db::write(ITEMS_TABLE, ITEMS_CACHE_KEY, &items);
                    *resolved.borrow_mut() = Some(CacheSource::Api);
                    Ok(items)
                }
                // Offline fallback: serve stale redb if present, else propagate the error.
                Err(err) => match db::read_stale::<Vec<Item>>(ITEMS_TABLE, ITEMS_CACHE_KEY) {
                    Some(items) => {
                        *resolved.borrow_mut() = Some(CacheSource::Disk);
                        Ok(items)
                    }
                    None => Err(err),
                },
            }
        });

    match outcome {
        Ok(entry) => {
            // is_fresh() == false means the value was already in Moka (an L1 hit).
            let source = if entry.is_fresh() {
                // If this caller was a dedup waiter, its loader closure never ran and
                // `resolved` is None; default to Api. This affects the debug `source`
                // label only — never the returned data.
                resolved.borrow().unwrap_or(CacheSource::Api)
            } else {
                CacheSource::Memory
            };
            let items = entry.into_value();
            let count = items.len();
            ItemsResult {
                items,
                source,
                count,
                error: None,
            }
        }
        // Only reached when the API failed AND no stale redb copy exists.
        Err(err) => ItemsResult {
            items: Vec::new(),
            source: CacheSource::Api,
            count: 0,
            error: Some(err.to_string()),
        },
    }
}

fn fetch_items_blocking() -> Result<Vec<Item>, String> {
    let (tx, rx) = mpsc::channel::<Result<Vec<Item>, String>>();

    std::thread::spawn(move || {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to build tokio runtime: {e}"))?;

            rt.block_on(async {
                let http_client = HTTP_CLIENT.clone();
                let client = MetaForgeClient::with_client(http_client);

                let q = ItemsQuery {
                    include_components: Some(true),
                    ..Default::default()
                };

                client
                    .items_all(&q)
                    .await
                    .map_err(|e| format!("API error fetching items: {e}"))
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

#[allow(dead_code)]
pub fn invalidate_items_cache() {
    ITEMS_CACHE.invalidate(&ITEMS_CACHE_KEY.to_string());
    db::remove(ITEMS_TABLE, ITEMS_CACHE_KEY);
}
