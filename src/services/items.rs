use crate::services::httpclientbuilder::HTTP_CLIENT;
use arc_api_rs::endpoints::items::ItemsQuery;
use arc_api_rs::models::Item;
use arc_api_rs::MetaForgeClient;
use moka::sync::Cache;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::LazyLock;
use std::time::Duration;

const CACHE_TTL_SECS: u64 = 900;
const ITEMS_CACHE_KEY: &str = "all_items";

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
pub struct ItemsResult {
    pub items: Vec<Item>,
    pub source: DataSource,
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
    if let Some(cached) = ITEMS_CACHE.get(&ITEMS_CACHE_KEY.to_string()) {
        let count = cached.len();
        return ItemsResult {
            items: cached,
            source: DataSource::Cache,
            count,
            error: None,
        };
    }

    match fetch_items_isolated().await {
        Ok(items) => {
            let count = items.len();
            ITEMS_CACHE.insert(ITEMS_CACHE_KEY.to_string(), items.clone());
            ItemsResult {
                items,
                source: DataSource::Api,
                count,
                error: None,
            }
        }
        Err(err) => ItemsResult {
            items: Vec::new(),
            source: DataSource::Api,
            count: 0,
            error: Some(err),
        },
    }
}

async fn fetch_items_isolated() -> Result<Vec<Item>, String> {
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
}
