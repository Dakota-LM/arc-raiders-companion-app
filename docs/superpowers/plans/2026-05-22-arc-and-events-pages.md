# Arc Page & Events Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a searchable, name-sortable, expandable **Arc (bots)** page and a live-countdown **Events** page to the Dioxus companion app, reusing the existing Materials/`ItemsView` patterns.

**Architecture:** Two data services (`bots.rs`, `events.rs`) mirror `services/items.rs` (moka 15-min cache + isolated-thread tokio fetch). Two card components (`ArcCard`, `EventCard`) mirror `ItemCard`. Two list components (`ArcsView`, `EventsView`) own UI state and fetch via `use_resource`. The Events page adds a 1-second clock tick + 60-second refetch loop; pure helper functions (`name_matches`, `format_remaining`, `partition_events`) are unit-tested with TDD.

**Tech Stack:** Rust, Dioxus 0.7.3 (native, `default = ["mobile"]`), `arc_api_rs` 0.2.x, `moka::sync`, `tokio`. Plain CSS with `--color-*` variables.

**Conventions (verified against the codebase):**
- Test command (host-buildable feature): `cargo test --no-default-features --features desktop <filter>`
- Check command: `cargo check --no-default-features --features desktop`
- Manual run: `dx serve --platform desktop` then click the page in the navbar.
- The crate has **zero existing tests** and no `[dev-dependencies]` — the new `#[cfg(test)]` modules use only `std` assertions (no new dev-deps).
- Routes (`#[route("/")] Events {}`, `#[route("/arcs")] Arcs {}`) and navbar links/icons **already exist**. Only the two stub views (`src/views/arcs.rs`, `src/views/events.rs`) need bodies.
- CSS per component: `const X_CSS: Asset = asset!("/assets/styling/x.css");` + `document::Link { rel: "stylesheet", href: X_CSS }` inside `rsx!`.

---

## Task 1: Arc data service (`services/bots.rs`)

**Files:**
- Create: `src/services/bots.rs`
- Modify: `src/services/mod.rs`

- [ ] **Step 1: Create the service file**

Create `src/services/bots.rs` (mirrors `services/items.rs` exactly, with `BotsQuery` sorting by name):

```rust
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
```

- [ ] **Step 2: Register the module**

In `src/services/mod.rs`, add below `pub mod items;`:

```rust
pub mod bots;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add src/services/bots.rs src/services/mod.rs
git commit -m "feat(arcs): add bots data service"
```

---

## Task 2: Arc name-matching logic — TDD (`components/arcs_view.rs`)

**Files:**
- Create: `src/components/arcs_view.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create the file with the function stub + failing tests**

Create `src/components/arcs_view.rs`:

```rust
use dioxus::prelude::*;

/// Returns true if `name` should be kept for the given search `query`.
/// An empty / whitespace-only query matches everything; otherwise matching is
/// a case-insensitive substring test.
fn name_matches(name: &str, query: &str) -> bool {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert!(name_matches("Rocketeer", ""));
        assert!(name_matches("Anything", "   "));
    }

    #[test]
    fn matches_case_insensitive_substring() {
        assert!(name_matches("Rocketeer", "rocket"));
        assert!(name_matches("rocketeer", "ROCKET"));
        assert!(name_matches("Tick Bot", "bot"));
    }

    #[test]
    fn non_match_returns_false() {
        assert!(!name_matches("Bombardier", "rocket"));
    }
}
```

Note: the `use dioxus::prelude::*;` import is unused until later tasks; that is acceptable for now (a warning, not an error). The component is added in Task 4.

- [ ] **Step 2: Register the module**

In `src/components/mod.rs`, add below `pub use items_view::ItemsView;`:

```rust
mod arcs_view;
```

(The matching `pub use arcs_view::ArcsView;` is added in Task 4.)

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --no-default-features --features desktop arcs_view::tests`
Expected: compiles, then tests FAIL / panic with `not implemented` (from `unimplemented!()`).

- [ ] **Step 4: Implement the function**

Replace the `name_matches` body:

```rust
fn name_matches(name: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&q)
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --no-default-features --features desktop arcs_view::tests`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 6: Commit**

```bash
git add src/components/arcs_view.rs src/components/mod.rs
git commit -m "feat(arcs): add name_matches search helper with tests"
```

---

## Task 3: Arc card component + CSS (`components/arc_card.rs`)

**Files:**
- Create: `assets/styling/arc_card.css`
- Create: `src/components/arc_card.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create the stylesheet**

Create `assets/styling/arc_card.css` (clone of `item_card.css` structure with the `.arc-card` prefix, no rarity borders):

```css
.arc-card {
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg-secondary);
  border: 0.0625rem solid var(--color-border);
  border-radius: 0.5rem;
  padding: 0.75rem;
  cursor: pointer;
  transition: background-color 0.15s ease;
}

.arc-card:hover {
  background-color: var(--color-bg-hover);
}

.arc-card__summary {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 0.75rem;
}

.arc-card__icon {
  width: 3rem;
  height: 3rem;
  object-fit: contain;
  flex-shrink: 0;
}

.arc-card__info {
  display: flex;
  flex-direction: column;
}

.arc-card__name {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text-primary);
}

.arc-card__details {
  display: none;
  flex-direction: column;
  gap: 0.75rem;
  margin-top: 0.75rem;
}

.arc-card__details--open {
  display: flex;
  animation: arc-detail-fade-in 0.2s ease;
}

.arc-card__image {
  width: 100%;
  max-height: 16rem;
  object-fit: contain;
  border-radius: 0.375rem;
}

.arc-card__description {
  font-size: 0.875rem;
  line-height: 1.5;
  color: var(--color-text-secondary);
  margin: 0;
}

@keyframes arc-detail-fade-in {
  from {
    opacity: 0;
    transform: translateY(-0.25rem);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

- [ ] **Step 2: Create the component**

Create `src/components/arc_card.rs` (mirrors `ItemCard`: `is_expanded: bool` + `on_toggle: EventHandler<String>`, parent owns the `expanded_id` signal):

```rust
use dioxus::prelude::*;

const ARC_CARD_CSS: Asset = asset!("/assets/styling/arc_card.css");

#[component]
pub fn ArcCard(
    id: String,
    name: String,
    icon_url: String,
    image_url: Option<String>,
    description: Vec<String>,
    is_expanded: bool,
    on_toggle: EventHandler<String>,
) -> Element {
    let details_class = if is_expanded {
        "arc-card__details arc-card__details--open"
    } else {
        "arc-card__details"
    };
    let card_id = id.clone();

    rsx! {
        document::Link { rel: "stylesheet", href: ARC_CARD_CSS }
        div {
            class: "arc-card",
            onclick: move |_| on_toggle.call(card_id.clone()),

            div { class: "arc-card__summary",
                img { class: "arc-card__icon", src: "{icon_url}", alt: "{name}" }
                div { class: "arc-card__info",
                    span { class: "arc-card__name", "{name}" }
                }
            }

            div { class: "{details_class}",
                if let Some(img) = image_url.clone() {
                    img { class: "arc-card__image", src: "{img}", alt: "{name}" }
                }
                for paragraph in description.iter() {
                    p { class: "arc-card__description", "{paragraph}" }
                }
            }
        }
    }
}
```

- [ ] **Step 3: Register the component**

In `src/components/mod.rs`, add (next to the other `mod`/`pub use` pairs):

```rust
mod arc_card;
pub use arc_card::ArcCard;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished` with no errors (an `unused: ArcCard` warning is fine; it is wired in Task 4).

- [ ] **Step 5: Commit**

```bash
git add assets/styling/arc_card.css src/components/arc_card.rs src/components/mod.rs
git commit -m "feat(arcs): add expandable ArcCard component"
```

---

## Task 4: Arc list view + wire the page (`components/arcs_view.rs`, `views/arcs.rs`)

**Files:**
- Create: `assets/styling/arcs_view.css`
- Modify: `src/components/arcs_view.rs`
- Modify: `src/components/mod.rs`
- Modify: `src/views/arcs.rs`

- [ ] **Step 1: Create the stylesheet**

Create `assets/styling/arcs_view.css`:

```css
.arcs-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.arcs-view__controls {
  display: flex;
  flex-direction: row;
  gap: 0.5rem;
  align-items: center;
}

.arcs-view__search {
  flex: 1;
  padding: 0.5rem 0.75rem;
  font-size: 0.875rem;
  color: var(--color-text-primary);
  background-color: var(--color-bg-secondary);
  border: 0.0625rem solid var(--color-border);
  border-radius: 0.5rem;
  outline: none;
}

.arcs-view__search::placeholder {
  color: var(--color-text-secondary);
}

.arcs-view__sort-btn {
  padding: 0.5rem 0.9rem;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  color: var(--color-text-secondary);
  background-color: var(--color-bg-secondary);
  border: 0.0625rem solid var(--color-border);
  border-radius: 0.5rem;
}

.arcs-view__sort-btn:hover {
  color: var(--color-text-primary);
}

.arcs-view__list {
  display: flex;
  flex-direction: column;
  gap: 2vw;
  padding-top: 2vw;
  flex: 1;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: var(--color-border) transparent;
}

.arcs-view__empty {
  padding: 2rem;
  text-align: center;
  color: var(--color-text-secondary);
}
```

- [ ] **Step 2: Add `filter_and_sort_bots` + the `ArcsView` component**

Edit `src/components/arcs_view.rs`. Replace the top `use dioxus::prelude::*;` line with the full import block, add `filter_and_sort_bots` after `name_matches`, and add the component. The file becomes:

```rust
use arc_api_rs::models::Bot;
use dioxus::prelude::*;

use super::{ArcCard, Spinner};
use crate::services::bots::get_all_bots;

const ARCS_VIEW_CSS: Asset = asset!("/assets/styling/arcs_view.css");

/// Returns true if `name` should be kept for the given search `query`.
/// An empty / whitespace-only query matches everything; otherwise matching is
/// a case-insensitive substring test.
fn name_matches(name: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&q)
}

/// Filter bots by name search, then sort alphabetically (A–Z, or Z–A if `sort_desc`).
fn filter_and_sort_bots(bots: &[Bot], search: &str, sort_desc: bool) -> Vec<Bot> {
    let mut out: Vec<Bot> = bots
        .iter()
        .filter(|b| name_matches(&b.name, search))
        .cloned()
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    if sort_desc {
        out.reverse();
    }
    out
}

#[component]
pub fn ArcsView() -> Element {
    let mut search = use_signal(String::new);
    let mut sort_desc = use_signal(|| false);
    let mut expanded_id: Signal<Option<String>> = use_signal(|| None);
    let mut is_loading = use_signal(|| true);

    let bots_res = use_resource(move || async move {
        is_loading.set(true);
        let result = get_all_bots().await;
        is_loading.set(false);
        result.bots
    });

    let loading = is_loading();
    let all = bots_res.read().clone().unwrap_or_default();
    let search_val = search();
    let desc = sort_desc();
    let filtered = filter_and_sort_bots(&all, &search_val, desc);
    let current_expanded = expanded_id();

    rsx! {
        document::Link { rel: "stylesheet", href: ARCS_VIEW_CSS }
        div { class: "arcs-view",
            div { class: "arcs-view__controls",
                input {
                    class: "arcs-view__search",
                    r#type: "text",
                    placeholder: "Search arcs...",
                    value: "{search_val}",
                    oninput: move |e| search.set(e.value()),
                }
                button {
                    class: "arcs-view__sort-btn",
                    onclick: move |_| {
                        let cur = sort_desc();
                        sort_desc.set(!cur);
                    },
                    if desc { "Z–A" } else { "A–Z" }
                }
            }

            if loading {
                Spinner { size: "2.5rem".to_string(), label: "Loading arcs...".to_string() }
            } else if filtered.is_empty() {
                div { class: "arcs-view__empty",
                    if all.is_empty() { "Failed to load arcs." } else { "No arcs match your search." }
                }
            } else {
                div { class: "arcs-view__list",
                    for bot in filtered.iter() {
                        ArcCard {
                            key: "{bot.id}",
                            id: bot.id.clone(),
                            name: bot.name.clone(),
                            icon_url: bot.icon.as_ref().map(|u| u.0.to_string()).unwrap_or_default(),
                            image_url: bot.image.as_ref().map(|u| u.0.to_string()),
                            description: bot.description.clone().unwrap_or_default(),
                            is_expanded: current_expanded.as_deref() == Some(bot.id.as_str()),
                            on_toggle: move |id: String| {
                                let current = expanded_id();
                                if current.as_deref() == Some(id.as_str()) {
                                    expanded_id.set(None);
                                } else {
                                    expanded_id.set(Some(id));
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert!(name_matches("Rocketeer", ""));
        assert!(name_matches("Anything", "   "));
    }

    #[test]
    fn matches_case_insensitive_substring() {
        assert!(name_matches("Rocketeer", "rocket"));
        assert!(name_matches("rocketeer", "ROCKET"));
        assert!(name_matches("Tick Bot", "bot"));
    }

    #[test]
    fn non_match_returns_false() {
        assert!(!name_matches("Bombardier", "rocket"));
    }
}
```

Note on field access: `Bot.icon`/`Bot.image` are `Option<UriString>` where `UriString(pub url::Url)`, so `.0.to_string()` yields the URL string. `Bot.description` is `Option<Vec<String>>`.

- [ ] **Step 3: Export the component**

In `src/components/mod.rs`, change the Arc view line so it reads:

```rust
mod arcs_view;
pub use arcs_view::ArcsView;
```

- [ ] **Step 4: Wire the page view**

Replace the body of `src/views/arcs.rs`:

```rust
use dioxus::prelude::*;

use crate::components::{ArcsView, PageLayout};

/// The Arcs page component that will be rendered when the current route is `[Route::Arcs]`
#[component]
pub fn Arcs() -> Element {
    rsx! {
        PageLayout {
            title: "Arcs",
            ArcsView {}
        }
    }
}
```

- [ ] **Step 5: Verify it compiles and tests still pass**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished`, no errors.
Run: `cargo test --no-default-features --features desktop arcs_view::tests`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 6: Manual verification**

Run: `dx serve --platform desktop`. Click **Arcs** in the navbar. Confirm: bots load as cards; typing in the search box filters by name; the A–Z/Z–A button reorders; clicking a card expands it to show the image + description and collapses on a second click; opening another card closes the first.

- [ ] **Step 7: Commit**

```bash
git add assets/styling/arcs_view.css src/components/arcs_view.rs src/components/mod.rs src/views/arcs.rs
git commit -m "feat(arcs): wire ArcsView page with search, sort, and expand"
```

---

## Task 5: Events data service (`services/events.rs`)

**Files:**
- Create: `src/services/events.rs`
- Modify: `src/services/mod.rs`

- [ ] **Step 1: Create the service file**

Create `src/services/events.rs` (same isolated-thread + moka pattern; caches the whole `EventsScheduleResponse`):

```rust
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
```

- [ ] **Step 2: Register the module**

In `src/services/mod.rs`, add below `pub mod bots;`:

```rust
pub mod events;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished`, no errors.

- [ ] **Step 4: Commit**

```bash
git add src/services/events.rs src/services/mod.rs
git commit -m "feat(events): add events schedule data service"
```

---

## Task 6: Event time logic — TDD (`components/event_card.rs`)

**Files:**
- Create: `src/components/event_card.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create the file with `EventState`, a `format_remaining` stub, and failing tests**

Create `src/components/event_card.rs`:

```rust
use dioxus::prelude::*;

/// Whether an event is currently running or has not started yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventState {
    Active,
    Upcoming,
}

/// Format a remaining duration in milliseconds as a compact countdown string.
/// >= 1 hour -> "Hh MMm"; otherwise "Mm SSs". Negative inputs clamp to zero.
fn format_remaining(ms: i64) -> String {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sub_hour_as_minutes_seconds() {
        assert_eq!(format_remaining(0), "0m 00s");
        assert_eq!(format_remaining(59_000), "0m 59s");
        assert_eq!(format_remaining(60_000), "1m 00s");
    }

    #[test]
    fn formats_hours_as_hours_minutes() {
        assert_eq!(format_remaining(3_600_000), "1h 00m");
        assert_eq!(format_remaining(4_980_000), "1h 23m");
    }

    #[test]
    fn clamps_negative_to_zero() {
        assert_eq!(format_remaining(-5_000), "0m 00s");
    }
}
```

- [ ] **Step 2: Register the module**

In `src/components/mod.rs`, add:

```rust
mod event_card;
```

(The matching `pub use event_card::EventCard;` is added in Task 8.)

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --no-default-features --features desktop event_card::tests`
Expected: compiles, then tests FAIL / panic with `not implemented`.

- [ ] **Step 4: Implement `format_remaining`**

Replace the body:

```rust
fn format_remaining(ms: i64) -> String {
    let total_secs = ms.max(0) / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, minutes)
    } else {
        format!("{}m {:02}s", minutes, seconds)
    }
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --no-default-features --features desktop event_card::tests`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 6: Commit**

```bash
git add src/components/event_card.rs src/components/mod.rs
git commit -m "feat(events): add EventState and format_remaining with tests"
```

---

## Task 7: Event partition logic — TDD (`components/events_view.rs`)

**Files:**
- Create: `src/components/events_view.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create the file with a `partition_events` stub + failing tests**

Create `src/components/events_view.rs`:

```rust
use arc_api_rs::models::ScheduledEvent;
use dioxus::prelude::*;

use crate::components::event_card::EventState;

/// Partition events relative to `now` (epoch ms):
/// - drops expired events (`end_time <= now`)
/// - returns active events first (sorted by `end_time` ascending),
///   then upcoming events (sorted by `start_time` ascending).
fn partition_events(events: &[ScheduledEvent], now: i64) -> Vec<(ScheduledEvent, EventState)> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(name: &str, start: i64, end: i64) -> ScheduledEvent {
        ScheduledEvent {
            name: name.to_string(),
            map: "Dam Battlegrounds".to_string(),
            icon: String::new(),
            start_time: start,
            end_time: end,
        }
    }

    #[test]
    fn drops_expired_events() {
        let now = 1000;
        let out = partition_events(&[ev("past", 0, 500), ev("live", 0, 2000)], now);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0.name, "live");
    }

    #[test]
    fn active_listed_before_upcoming() {
        let now = 1000;
        let out = partition_events(&[ev("soon", 2000, 3000), ev("live", 500, 2000)], now);
        assert_eq!(out[0].0.name, "live");
        assert_eq!(out[0].1, EventState::Active);
        assert_eq!(out[1].0.name, "soon");
        assert_eq!(out[1].1, EventState::Upcoming);
    }

    #[test]
    fn active_sorted_by_end_time() {
        let now = 1000;
        let out = partition_events(&[ev("ends_late", 0, 5000), ev("ends_soon", 0, 2000)], now);
        assert_eq!(out[0].0.name, "ends_soon");
        assert_eq!(out[1].0.name, "ends_late");
    }

    #[test]
    fn upcoming_sorted_by_start_time() {
        let now = 1000;
        let out = partition_events(&[ev("later", 5000, 6000), ev("sooner", 2000, 3000)], now);
        assert_eq!(out[0].0.name, "sooner");
        assert_eq!(out[1].0.name, "later");
    }

    #[test]
    fn boundaries_start_equals_now_is_active_end_equals_now_is_expired() {
        let now = 1000;
        let out = partition_events(&[ev("starts_now", 1000, 2000), ev("ends_now", 0, 1000)], now);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0.name, "starts_now");
        assert_eq!(out[0].1, EventState::Active);
    }
}
```

- [ ] **Step 2: Register the module**

In `src/components/mod.rs`, add:

```rust
mod events_view;
```

(The matching `pub use events_view::EventsView;` is added in Task 9.)

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --no-default-features --features desktop events_view::tests`
Expected: compiles, then tests FAIL / panic with `not implemented`.

- [ ] **Step 4: Implement `partition_events`**

Replace the body:

```rust
fn partition_events(events: &[ScheduledEvent], now: i64) -> Vec<(ScheduledEvent, EventState)> {
    let mut active: Vec<ScheduledEvent> = Vec::new();
    let mut upcoming: Vec<ScheduledEvent> = Vec::new();
    for e in events {
        if e.end_time <= now {
            continue; // expired
        }
        if e.start_time <= now {
            active.push(e.clone());
        } else {
            upcoming.push(e.clone());
        }
    }
    active.sort_by_key(|e| e.end_time);
    upcoming.sort_by_key(|e| e.start_time);
    active
        .into_iter()
        .map(|e| (e, EventState::Active))
        .chain(upcoming.into_iter().map(|e| (e, EventState::Upcoming)))
        .collect()
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --no-default-features --features desktop events_view::tests`
Expected: `test result: ok. 5 passed`.

- [ ] **Step 6: Commit**

```bash
git add src/components/events_view.rs src/components/mod.rs
git commit -m "feat(events): add partition_events with tests"
```

---

## Task 8: Event card component + CSS (`components/event_card.rs`)

**Files:**
- Create: `assets/styling/event_card.css`
- Modify: `src/components/event_card.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create the stylesheet**

Create `assets/styling/event_card.css`:

```css
.event-card {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 0.75rem;
  background-color: var(--color-bg-secondary);
  border: 0.0625rem solid var(--color-border);
  border-radius: 0.5rem;
  padding: 0.75rem;
  transition: opacity 0.2s ease;
}

.event-card--upcoming {
  opacity: 0.55;
}

.event-card__icon {
  width: 3rem;
  height: 3rem;
  object-fit: contain;
  flex-shrink: 0;
}

.event-card__info {
  display: flex;
  flex-direction: column;
  flex: 1;
}

.event-card__name {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text-primary);
}

.event-card__map {
  font-size: 0.8rem;
  color: var(--color-text-secondary);
}

.event-card__countdown {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-accent);
  white-space: nowrap;
}
```

- [ ] **Step 2: Add the `EventCard` component**

Edit `src/components/event_card.rs`. Add the CSS asset const after the existing `use dioxus::prelude::*;`, and append the component below `format_remaining` (keep `EventState`, `format_remaining`, and the test module unchanged). Add at the top, after the import:

```rust
const EVENT_CARD_CSS: Asset = asset!("/assets/styling/event_card.css");
```

Then append after the `format_remaining` function:

```rust
#[component]
pub fn EventCard(
    name: String,
    map: String,
    icon_url: String,
    state: EventState,
    now: i64,
    start_time: i64,
    end_time: i64,
) -> Element {
    let remaining_ms = match state {
        EventState::Active => end_time - now,
        EventState::Upcoming => start_time - now,
    };
    let card_class = match state {
        EventState::Active => "event-card",
        EventState::Upcoming => "event-card event-card--upcoming",
    };
    let label = match state {
        EventState::Active => format!("Ends in {}", format_remaining(remaining_ms)),
        EventState::Upcoming => format!("Starts in {}", format_remaining(remaining_ms)),
    };

    rsx! {
        document::Link { rel: "stylesheet", href: EVENT_CARD_CSS }
        div { class: "{card_class}",
            img { class: "event-card__icon", src: "{icon_url}", alt: "{name}" }
            div { class: "event-card__info",
                span { class: "event-card__name", "{name}" }
                span { class: "event-card__map", "{map}" }
            }
            span { class: "event-card__countdown", "{label}" }
        }
    }
}
```

- [ ] **Step 3: Export the component**

In `src/components/mod.rs`, change the event-card line so it reads:

```rust
mod event_card;
pub use event_card::EventCard;
```

- [ ] **Step 4: Verify it compiles and tests still pass**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished`, no errors (an `unused: EventCard` warning is fine; wired in Task 9).
Run: `cargo test --no-default-features --features desktop event_card::tests`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Commit**

```bash
git add assets/styling/event_card.css src/components/event_card.rs src/components/mod.rs
git commit -m "feat(events): add EventCard component with live countdown label"
```

---

## Task 9: Events list view + wire the page (`components/events_view.rs`, `views/events.rs`)

**Files:**
- Create: `assets/styling/events_view.css`
- Modify: `src/components/events_view.rs`
- Modify: `src/components/mod.rs`
- Modify: `src/views/events.rs`

- [ ] **Step 1: Create the stylesheet**

Create `assets/styling/events_view.css`:

```css
.events-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.events-view__list {
  display: flex;
  flex-direction: column;
  gap: 2vw;
  padding-top: 2vw;
  flex: 1;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: var(--color-border) transparent;
}

.events-view__empty {
  padding: 2rem;
  text-align: center;
  color: var(--color-text-secondary);
}
```

- [ ] **Step 2: Add the clock loop + `EventsView` component**

Edit `src/components/events_view.rs`. Update the imports, add `now_ms()` and the CSS const, and append the component. Keep `partition_events` and the test module unchanged. The non-test portion of the file becomes:

```rust
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arc_api_rs::models::ScheduledEvent;
use dioxus::prelude::*;

use super::{EventCard, Spinner};
use crate::components::event_card::EventState;
use crate::services::events::get_event_schedule;

const EVENTS_VIEW_CSS: Asset = asset!("/assets/styling/events_view.css");

/// Current wall-clock time in epoch milliseconds.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Partition events relative to `now` (epoch ms):
/// - drops expired events (`end_time <= now`)
/// - returns active events first (sorted by `end_time` ascending),
///   then upcoming events (sorted by `start_time` ascending).
fn partition_events(events: &[ScheduledEvent], now: i64) -> Vec<(ScheduledEvent, EventState)> {
    let mut active: Vec<ScheduledEvent> = Vec::new();
    let mut upcoming: Vec<ScheduledEvent> = Vec::new();
    for e in events {
        if e.end_time <= now {
            continue; // expired
        }
        if e.start_time <= now {
            active.push(e.clone());
        } else {
            upcoming.push(e.clone());
        }
    }
    active.sort_by_key(|e| e.end_time);
    upcoming.sort_by_key(|e| e.start_time);
    active
        .into_iter()
        .map(|e| (e, EventState::Active))
        .chain(upcoming.into_iter().map(|e| (e, EventState::Upcoming)))
        .collect()
}

#[component]
pub fn EventsView() -> Element {
    let mut now = use_signal(now_ms);
    let mut refresh = use_signal(|| 0u32);

    // Local clock: tick every second; trigger an API refetch every 60 ticks.
    use_future(move || async move {
        let mut tick: u64 = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            now.set(now_ms());
            tick += 1;
            if tick % 60 == 0 {
                let cur = refresh();
                refresh.set(cur.wrapping_add(1));
            }
        }
    });

    let events_res = use_resource(move || async move {
        let _ = refresh(); // subscribe: re-runs the fetch whenever `refresh` changes
        get_event_schedule().await.events
    });

    let resource = events_res.read();
    let loading = resource.is_none();
    let all = resource.clone().unwrap_or_default();
    let now_val = now();
    let visible = partition_events(&all, now_val);

    rsx! {
        document::Link { rel: "stylesheet", href: EVENTS_VIEW_CSS }
        div { class: "events-view",
            if loading {
                Spinner { size: "2.5rem".to_string(), label: "Loading events...".to_string() }
            } else if visible.is_empty() {
                div { class: "events-view__empty",
                    if all.is_empty() { "Failed to load events." } else { "No active or upcoming events." }
                }
            } else {
                div { class: "events-view__list",
                    for (event, state) in visible.iter() {
                        EventCard {
                            key: "{event.name}-{event.start_time}",
                            name: event.name.clone(),
                            map: event.map.clone(),
                            icon_url: event.icon.clone(),
                            state: *state,
                            now: now_val,
                            start_time: event.start_time,
                            end_time: event.end_time,
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 3: Export the component**

In `src/components/mod.rs`, change the events-view line so it reads:

```rust
mod events_view;
pub use events_view::EventsView;
```

- [ ] **Step 4: Wire the page view**

Replace the body of `src/views/events.rs`:

```rust
use dioxus::prelude::*;

use crate::components::{EventsView, PageLayout};

/// The Events page component that will be rendered when the current route is `[Route::Events]`
#[component]
pub fn Events() -> Element {
    rsx! {
        PageLayout {
            title: "Events",
            EventsView {}
        }
    }
}
```

- [ ] **Step 5: Verify it compiles and tests still pass**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished`, no errors.
Run: `cargo test --no-default-features --features desktop events_view::tests`
Expected: `test result: ok. 5 passed`.

- [ ] **Step 6: Manual verification**

Run: `dx serve --platform desktop`. The Events page is the home route (`/`). Confirm:
- Active events render at full opacity; upcoming events are visibly faded (~0.55 opacity); no expired events appear.
- Each card's countdown ticks down once per second ("Ends in …" for active, "Starts in …" for upcoming).
- Leave it running ~60s and confirm a refetch happens without the countdown resetting incorrectly.

  **If the countdown does NOT tick (or the app panics with a timer/reactor error):** Dioxus's `use_future` polling context may not provide tokio's timer driver. Fix by adding a runtime-agnostic timer:
  1. `cargo add futures-timer`
  2. In `events_view.rs`, replace `tokio::time::sleep(Duration::from_secs(1)).await;` with `futures_timer::Delay::new(Duration::from_secs(1)).await;` (drop the `tokio` reference; `Duration` is already imported).
  3. Re-run Steps 5–6.

- [ ] **Step 7: Commit**

```bash
git add assets/styling/events_view.css src/components/events_view.rs src/components/mod.rs src/views/events.rs
git commit -m "feat(events): wire EventsView page with live clock and refetch"
```

---

## Final Verification

- [ ] Run the full test suite: `cargo test --no-default-features --features desktop`
  Expected: all tests pass (3 in `arcs_view`, 3 in `event_card`, 5 in `events_view`).
- [ ] Run `cargo check --no-default-features --features desktop` — no errors.
- [ ] `dx serve --platform desktop`: navigate Materials, Traders, **Arcs**, **Events** — confirm Arcs (search/sort/expand) and Events (live countdown, faded upcoming, no expired) work and that Materials/Traders are unchanged.

## Notes / Decisions (from the spec)

- **Native-only** for the clock (`std::time::SystemTime` + `tokio::time::sleep`, with the `futures-timer` fallback documented in Task 9). A future web/WASM build would need cfg-gated `js_sys::Date` + `gloo-timers`.
- **Refetch interval = 60s**; **active events sorted by `end_time` asc**; **Arc search/sort are client-side** over the cached list.
- No server-clock-skew correction (device clock is "now"). No bot loot display (not on the `Bot` model). No event filtering beyond active/upcoming partitioning.
