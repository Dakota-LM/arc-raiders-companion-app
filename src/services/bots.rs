use crate::services::httpclientbuilder::HTTP_CLIENT;
use arc_api_rs::endpoints::bots::BotsQuery;
use arc_api_rs::models::Bot;
use arc_api_rs::MetaForgeClient;
use moka::sync::Cache;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::LazyLock;
use std::time::Duration;

const CACHE_TTL_SECS: u64 = 900;
const BOTS_CACHE_KEY: &str = "all_bots";

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
    if let Some(cached) = BOTS_CACHE.get(&BOTS_CACHE_KEY.to_string()) {
        let count = cached.len();
        return BotsResult {
            bots: cached,
            source: DataSource::Cache,
            count,
            error: None,
        };
    }

    match fetch_bots_isolated().await {
        Ok(bots) => {
            let count = bots.len();
            BOTS_CACHE.insert(BOTS_CACHE_KEY.to_string(), bots.clone());
            BotsResult {
                bots,
                source: DataSource::Api,
                count,
                error: None,
            }
        }
        Err(err) => BotsResult {
            bots: Vec::new(),
            source: DataSource::Api,
            count: 0,
            error: Some(err),
        },
    }
}

async fn fetch_bots_isolated() -> Result<Vec<Bot>, String> {
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
}
