use crate::services::httpclientbuilder::HTTP_CLIENT;
use arc_api_rs::models::{EventsScheduleResponse, ScheduledEvent};
use arc_api_rs::MetaForgeClient;
use moka::sync::Cache;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::LazyLock;
use std::time::Duration;

const CACHE_TTL_SECS: u64 = 900;
const EVENTS_CACHE_KEY: &str = "events_schedule";

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
pub struct EventsResult {
    pub events: Vec<ScheduledEvent>,
    pub cached_at: i64,
    pub source: DataSource,
    pub count: usize,
    pub error: Option<String>,
}

static EVENTS_CACHE: LazyLock<Cache<String, EventsScheduleResponse>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(4)
        .time_to_live(Duration::from_secs(CACHE_TTL_SECS))
        .build()
});

/// Fetch the scheduled-events list, preferring the in-memory cache.
pub async fn get_event_schedule() -> EventsResult {
    if let Some(cached) = EVENTS_CACHE.get(&EVENTS_CACHE_KEY.to_string()) {
        return EventsResult {
            count: cached.data.len(),
            events: cached.data.clone(),
            cached_at: cached.cached_at,
            source: DataSource::Cache,
            error: None,
        };
    }
    match fetch_events_isolated().await {
        Ok(resp) => {
            let count = resp.data.len();
            let cached_at = resp.cached_at;
            let events = resp.data.clone();
            EVENTS_CACHE.insert(EVENTS_CACHE_KEY.to_string(), resp);
            EventsResult {
                events,
                cached_at,
                source: DataSource::Api,
                count,
                error: None,
            }
        }
        Err(err) => EventsResult {
            events: Vec::new(),
            cached_at: 0,
            source: DataSource::Api,
            count: 0,
            error: Some(err),
        },
    }
}

async fn fetch_events_isolated() -> Result<EventsScheduleResponse, String> {
    let (tx, rx) = mpsc::channel::<Result<EventsScheduleResponse, String>>();
    std::thread::spawn(move || {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to build tokio runtime: {e}"))?;
            rt.block_on(async {
                let client = MetaForgeClient::with_client(HTTP_CLIENT.clone());
                client
                    .events_schedule()
                    .await
                    .map_err(|e| format!("API error fetching events: {e}"))
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
