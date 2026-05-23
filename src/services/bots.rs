use crate::services::db;
use crate::services::httpclientbuilder::HTTP_CLIENT;
use arc_api_rs::endpoints::bots::BotsQuery;
use arc_api_rs::models::Bot;
use arc_api_rs::MetaForgeClient;
use moka::sync::Cache;
use redb::TableDefinition;
use std::cell::RefCell;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::LazyLock;
use std::time::Duration;

const CACHE_TTL_SECS: u64 = 900;
const BOTS_CACHE_KEY: &str = "all_bots";
const BOTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("bots");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataSource {
    Api,
    Cache,
}

impl fmt::Display for DataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataSource::Api => write!(f, "API"),
            DataSource::Cache => write!(f, "Cache"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BotsResult {
    pub bots: Vec<Bot>,
    pub source: DataSource,
    pub count: usize,
    pub error: Option<String>,
}

static BOTS_CACHE: LazyLock<Cache<String, Vec<Bot>>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(4)
        .time_to_live(Duration::from_secs(CACHE_TTL_SECS))
        .build()
});

/// Fetch all bots ("Arcs"), preferring the in-memory cache.
pub async fn get_all_bots() -> BotsResult {
    let resolved: RefCell<Option<DataSource>> = RefCell::new(None);

    let outcome = BOTS_CACHE
        .entry(BOTS_CACHE_KEY.to_string())
        .or_try_insert_with(|| -> Result<Vec<Bot>, String> {
            // L2: fresh redb
            if let Some(bots) = db::read_fresh::<Vec<Bot>>(
                BOTS_TABLE,
                BOTS_CACHE_KEY,
                Duration::from_secs(CACHE_TTL_SECS),
            ) {
                *resolved.borrow_mut() = Some(DataSource::Cache);
                return Ok(bots);
            }
            // Source: API (write-through)
            match fetch_bots_blocking() {
                Ok(bots) => {
                    db::write(BOTS_TABLE, BOTS_CACHE_KEY, &bots);
                    *resolved.borrow_mut() = Some(DataSource::Api);
                    Ok(bots)
                }
                // Offline fallback: stale redb
                Err(err) => match db::read_stale::<Vec<Bot>>(BOTS_TABLE, BOTS_CACHE_KEY) {
                    Some(bots) => {
                        *resolved.borrow_mut() = Some(DataSource::Cache);
                        Ok(bots)
                    }
                    None => Err(err),
                },
            }
        });

    match outcome {
        Ok(entry) => {
            let source = if entry.is_fresh() {
                // Dedup waiters never ran the loader; default to Api (debug label only).
                resolved.borrow().clone().unwrap_or(DataSource::Api)
            } else {
                DataSource::Cache
            };
            let bots = entry.into_value();
            let count = bots.len();
            BotsResult { bots, source, count, error: None }
        }
        Err(err) => BotsResult {
            bots: Vec::new(),
            source: DataSource::Api,
            count: 0,
            error: Some(err.to_string()),
        },
    }
}

fn fetch_bots_blocking() -> Result<Vec<Bot>, String> {
    let (tx, rx) = mpsc::channel::<Result<Vec<Bot>, String>>();

    std::thread::spawn(move || {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to build tokio runtime: {e}"))?;

            rt.block_on(async {
                let http_client = HTTP_CLIENT.clone();
                let client = MetaForgeClient::with_client(http_client);

                let q = BotsQuery {
                    sort_by: Some("name".to_string()),
                    sort_order: Some("asc".to_string()),
                    ..Default::default()
                };

                client
                    .bots_all(&q)
                    .await
                    .map_err(|e| format!("API error fetching bots: {e}"))
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
pub fn invalidate_bots_cache() {
    BOTS_CACHE.invalidate(&BOTS_CACHE_KEY.to_string());
    db::remove(BOTS_TABLE, BOTS_CACHE_KEY);
}
