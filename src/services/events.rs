use crate::services::db;
use crate::services::httpclientbuilder::HTTP_CLIENT;
use arc_api_rs::models::{EventsScheduleResponse, ScheduledEvent};
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
const EVENTS_CACHE_KEY: &str = "events_schedule";
const EVENTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("events");

#[derive(Debug, Clone)]
pub struct EventsResult {
    pub events: Vec<ScheduledEvent>,
    pub cached_at: i64,
    pub source: CacheSource,
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
    let resolved: RefCell<Option<CacheSource>> = RefCell::new(None);

    let outcome = EVENTS_CACHE
        .entry(EVENTS_CACHE_KEY.to_string())
        .or_try_insert_with(|| -> Result<EventsScheduleResponse, String> {
            // L2: fresh redb
            if let Some(resp) = db::read_fresh::<EventsScheduleResponse>(
                EVENTS_TABLE,
                EVENTS_CACHE_KEY,
                Duration::from_secs(CACHE_TTL_SECS),
            ) {
                *resolved.borrow_mut() = Some(CacheSource::Disk);
                return Ok(resp);
            }
            // Source: API (write-through)
            match fetch_events_blocking() {
                Ok(resp) => {
                    db::write(EVENTS_TABLE, EVENTS_CACHE_KEY, &resp);
                    *resolved.borrow_mut() = Some(CacheSource::Api);
                    Ok(resp)
                }
                // Offline fallback: stale redb
                Err(err) => match db::read_stale::<EventsScheduleResponse>(EVENTS_TABLE, EVENTS_CACHE_KEY) {
                    Some(resp) => {
                        *resolved.borrow_mut() = Some(CacheSource::Disk);
                        Ok(resp)
                    }
                    None => Err(err),
                },
            }
        });

    match outcome {
        Ok(entry) => {
            let source = if entry.is_fresh() {
                // Dedup waiters never ran the loader; default to Api (debug label only).
                resolved.borrow().unwrap_or(CacheSource::Api)
            } else {
                CacheSource::Memory
            };
            let resp = entry.into_value();
            let count = resp.data.len();
            let cached_at = resp.cached_at;
            EventsResult { events: resp.data, cached_at, source, count, error: None }
        }
        Err(err) => EventsResult {
            events: Vec::new(),
            cached_at: 0,
            source: CacheSource::Api,
            count: 0,
            error: Some(err.to_string()),
        },
    }
}

fn fetch_events_blocking() -> Result<EventsScheduleResponse, String> {
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

/// Invalidate the events cache in both tiers (Moka + redb).
#[allow(dead_code)]
pub fn invalidate_events_cache() {
    EVENTS_CACHE.invalidate(&EVENTS_CACHE_KEY.to_string());
    db::remove(EVENTS_TABLE, EVENTS_CACHE_KEY);
}
