# Tier-Aware Cache Badge + Clear-Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface which cache tier (API / Memory / Disk / Fallback) served each data page via a reusable badge, and add a Settings → Advanced → Clear cache control that flushes both Moka and redb.

**Architecture:** A shared `CacheSource` enum replaces the four per-service `DataSource` enums; each service maps its cascade branches to it (Moka hit → Memory, redb → Disk, API → Api, traders fallback → Fallback). A reusable `CacheBadge` component renders the tier as a colored chip on all four data pages, replacing the two inline string-matching banners. Settings gains an Advanced section whose Clear cache button calls every `invalidate_*_cache()` (each already flushes Moka + redb) and shows a brief inline confirmation.

**Tech Stack:** Rust, Dioxus 0.7 (`#[component]`, `use_signal`, `use_resource`, `spawn`), the existing `services::db` redb layer.

**Reference spec:** `docs/superpowers/specs/2026-05-22-cache-indicator-design.md`

**Branch:** Execute on a feature branch (e.g. `feat/cache-indicator`), not `main`.

**Canonical commands** (the default `mobile` feature does not link on a desktop host):
- Test: `cargo test --no-default-features --features desktop`
- Lint: `cargo clippy --no-default-features --features desktop`

---

## File Structure

- **Create** `src/services/source.rs` — the shared `CacheSource` enum + `label()`/`css_class()`/`Display`.
- **Create** `src/components/cache_badge.rs` — the `CacheBadge` component.
- **Create** `assets/styling/cache_badge.css` — badge styles (`--api/--memory/--disk/--fallback`).
- **Create** `assets/styling/settings.css` — Advanced section + clear button styles.
- **Modify** `src/services/mod.rs` — `pub mod source;`.
- **Modify** `src/services/{items,bots,events,traders}.rs` — swap `DataSource` → `CacheSource`; add `invalidate_events_cache()`.
- **Modify** `src/components/mod.rs` — export `CacheBadge`.
- **Modify** `src/components/{items_view,trader_view,arcs_view,events_view}.rs` — render `CacheBadge`.
- **Modify** `assets/styling/{items_view,trader_view,arcs_view,events_view}.css` — drop dead banner rules, add badge wrappers.
- **Modify** `src/views/settings.rs` — Advanced section + Clear cache.

---

### Task 1: Shared `CacheSource` enum (TDD)

**Files:**
- Create: `src/services/source.rs`
- Modify: `src/services/mod.rs`

- [ ] **Step 1: Write the failing tests + skeleton**

Create `src/services/source.rs`:

```rust
//! Shared description of which cache tier served a value. Produced by every data
//! service and rendered by the `CacheBadge` UI component.

use std::fmt;

/// Which tier of the cache cascade served a piece of data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSource {
    /// Fetched live from the MetaForge API.
    Api,
    /// Served from the Moka in-memory L1 cache.
    Memory,
    /// Served from the redb on-disk L2 cache (fresh or stale).
    Disk,
    /// Traders' hardcoded fallback list (API and both caches unavailable).
    Fallback,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_matches_each_variant() {
        assert_eq!(CacheSource::Api.label(), "API");
        assert_eq!(CacheSource::Memory.label(), "Memory");
        assert_eq!(CacheSource::Disk.label(), "Disk");
        assert_eq!(CacheSource::Fallback.label(), "Fallback");
    }

    #[test]
    fn css_class_matches_each_variant() {
        assert_eq!(CacheSource::Api.css_class(), "api");
        assert_eq!(CacheSource::Memory.css_class(), "memory");
        assert_eq!(CacheSource::Disk.css_class(), "disk");
        assert_eq!(CacheSource::Fallback.css_class(), "fallback");
    }

    #[test]
    fn display_uses_label() {
        assert_eq!(format!("{}", CacheSource::Disk), "Disk");
    }
}
```

Add `pub mod source;` to `src/services/mod.rs` (after the existing `pub mod db;` line) so the test compiles.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --no-default-features --features desktop services::source`
Expected: FAIL — compile error, `no method named label`/`css_class` and `CacheSource` doesn't implement `Display`.

- [ ] **Step 3: Implement `label`, `css_class`, `Display`**

In `src/services/source.rs`, insert between the enum and the `#[cfg(test)]` module:

```rust
impl CacheSource {
    /// Human-readable label for the UI badge.
    pub fn label(&self) -> &'static str {
        match self {
            CacheSource::Api => "API",
            CacheSource::Memory => "Memory",
            CacheSource::Disk => "Disk",
            CacheSource::Fallback => "Fallback",
        }
    }

    /// CSS modifier suffix for the `cache-badge--{}` class.
    pub fn css_class(&self) -> &'static str {
        match self {
            CacheSource::Api => "api",
            CacheSource::Memory => "memory",
            CacheSource::Disk => "disk",
            CacheSource::Fallback => "fallback",
        }
    }
}

impl fmt::Display for CacheSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --no-default-features --features desktop services::source`
Expected: PASS — 3 tests in `services::source::tests`.

- [ ] **Step 5: Commit**

```bash
git add src/services/source.rs src/services/mod.rs
git commit -m "feat(cache): add shared CacheSource enum (API/Memory/Disk/Fallback)"
```

---

### Task 2: `CacheBadge` component + CSS

**Files:**
- Create: `src/components/cache_badge.rs`
- Create: `assets/styling/cache_badge.css`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create the component**

Create `src/components/cache_badge.rs`:

```rust
use dioxus::prelude::*;

use crate::services::source::CacheSource;

const CACHE_BADGE_CSS: Asset = asset!("/assets/styling/cache_badge.css");

/// A small colored chip showing which cache tier served some data.
///
/// # Props
/// - `source`: the tier (API / Memory / Disk / Fallback) — drives the color and text.
/// - `count`: optional item count, appended as `· N`.
/// - `label`: optional leading label, e.g. `"Items"` → `"Items: Memory · 18"`.
/// - `error`: optional error message, appended after the source.
#[component]
pub fn CacheBadge(
    source: CacheSource,
    #[props(default)] count: Option<usize>,
    #[props(default)] label: Option<String>,
    #[props(default)] error: Option<String>,
) -> Element {
    let class = format!("cache-badge cache-badge--{}", source.css_class());

    let mut text = String::new();
    if let Some(label) = &label {
        text.push_str(label);
        text.push_str(": ");
    }
    text.push_str(source.label());
    if let Some(count) = count {
        text.push_str(&format!(" · {count}"));
    }
    if let Some(error) = &error {
        text.push_str(&format!(" · {error}"));
    }

    rsx! {
        document::Link { rel: "stylesheet", href: CACHE_BADGE_CSS }
        div { class: "{class}", "{text}" }
    }
}
```

- [ ] **Step 2: Create the CSS**

Create `assets/styling/cache_badge.css`:

```css
/* Cache source badge — shows which tier (API / Memory / Disk / Fallback) served data. */
.cache-badge {
    font-size: 0.7rem;
    font-family: monospace;
    padding: 0.35rem 0.75rem;
    border-radius: 0.35rem;
    text-align: center;
    animation: cache-badge-fade-in 0.3s ease both;
}

@keyframes cache-badge-fade-in {
    from { opacity: 0; transform: translateY(-2px); }
    to   { opacity: 1; transform: translateY(0); }
}

.cache-badge--api {
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
    border: 1px solid rgba(34, 197, 94, 0.3);
}
.cache-badge--memory {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
    border: 1px solid rgba(59, 130, 246, 0.3);
}
.cache-badge--disk {
    background: rgba(168, 85, 247, 0.15);
    color: #c084fc;
    border: 1px solid rgba(168, 85, 247, 0.3);
}
.cache-badge--fallback {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
    border: 1px solid rgba(245, 158, 11, 0.3);
}

:root.light .cache-badge--api {
    background: rgba(34, 197, 94, 0.1);
    color: #16a34a;
    border-color: rgba(34, 197, 94, 0.25);
}
:root.light .cache-badge--memory {
    background: rgba(59, 130, 246, 0.1);
    color: #2563eb;
    border-color: rgba(59, 130, 246, 0.25);
}
:root.light .cache-badge--disk {
    background: rgba(168, 85, 247, 0.1);
    color: #7c3aed;
    border-color: rgba(168, 85, 247, 0.25);
}
:root.light .cache-badge--fallback {
    background: rgba(245, 158, 11, 0.1);
    color: #b45309;
    border-color: rgba(245, 158, 11, 0.25);
}
```

- [ ] **Step 3: Export the component**

In `src/components/mod.rs`, add (e.g. after the `spinner` block):

```rust
mod cache_badge;
pub use cache_badge::CacheBadge;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check --no-default-features --features desktop`
Expected: `Finished` (a `CacheBadge`/`CacheSource` "never used" warning is fine until Task 4 wires it).

- [ ] **Step 5: Commit**

```bash
git add src/components/cache_badge.rs src/components/mod.rs assets/styling/cache_badge.css
git commit -m "feat(cache): add reusable CacheBadge component + styles"
```

---

### Task 3: Swap services to `CacheSource` + add `invalidate_events_cache`

Each service drops its own `DataSource` enum (+ `Display` impl + `use std::fmt;`), imports `CacheSource`, retypes its `*Result.source` field, and maps each cascade branch to a tier. The views still compile because `CacheSource` implements `Display` (they format the source string until Task 4).

**Files:** Modify `src/services/items.rs`, `src/services/bots.rs`, `src/services/events.rs`, `src/services/traders.rs`.

- [ ] **Step 1: items.rs**

Remove the line `use std::fmt;`. Add (with the other `use` lines): `use crate::services::source::CacheSource;`

Delete the entire `DataSource` enum and its `impl fmt::Display for DataSource { … }` block.

Change the result field: `pub source: DataSource,` → `pub source: CacheSource,`

In `get_all_items`, apply these exact replacements:
- `let resolved: RefCell<Option<DataSource>> = RefCell::new(None);` → `let resolved: RefCell<Option<CacheSource>> = RefCell::new(None);`
- The fresh-redb branch `*resolved.borrow_mut() = Some(DataSource::Cache);` → `*resolved.borrow_mut() = Some(CacheSource::Disk);`
- The API-success branch `*resolved.borrow_mut() = Some(DataSource::Api);` → `*resolved.borrow_mut() = Some(CacheSource::Api);`
- The stale-redb branch `*resolved.borrow_mut() = Some(DataSource::Cache);` → `*resolved.borrow_mut() = Some(CacheSource::Disk);`
- The source selection block:
  ```rust
  let source = if entry.is_fresh() {
      resolved.borrow().clone().unwrap_or(DataSource::Api)
  } else {
      DataSource::Cache
  };
  ```
  →
  ```rust
  let source = if entry.is_fresh() {
      resolved.borrow().unwrap_or(CacheSource::Api)
  } else {
      CacheSource::Memory
  };
  ```
- The error result `source: DataSource::Api,` → `source: CacheSource::Api,`

- [ ] **Step 2: bots.rs**

Identical shape to items. Remove `use std::fmt;`, add `use crate::services::source::CacheSource;`, delete the `DataSource` enum + `Display` impl, change `pub source: DataSource,` → `pub source: CacheSource,`, then in `get_all_bots`:
- `RefCell<Option<DataSource>>` → `RefCell<Option<CacheSource>>`
- fresh-redb `Some(DataSource::Cache)` → `Some(CacheSource::Disk)`
- API `Some(DataSource::Api)` → `Some(CacheSource::Api)`
- stale-redb `Some(DataSource::Cache)` → `Some(CacheSource::Disk)`
- source block:
  ```rust
  let source = if entry.is_fresh() {
      resolved.borrow().unwrap_or(CacheSource::Api)
  } else {
      CacheSource::Memory
  };
  ```
- error result `source: CacheSource::Api,`

- [ ] **Step 3: events.rs**

Same shape. Remove `use std::fmt;`, add `use crate::services::source::CacheSource;`, delete `DataSource` enum + `Display`, change `pub source: DataSource,` → `pub source: CacheSource,`, then in `get_event_schedule`:
- `RefCell<Option<DataSource>>` → `RefCell<Option<CacheSource>>`
- fresh-redb `Some(DataSource::Cache)` → `Some(CacheSource::Disk)`
- API `Some(DataSource::Api)` → `Some(CacheSource::Api)`
- stale-redb `Some(DataSource::Cache)` → `Some(CacheSource::Disk)`
- source block:
  ```rust
  let source = if entry.is_fresh() {
      resolved.borrow().unwrap_or(CacheSource::Api)
  } else {
      CacheSource::Memory
  };
  ```
- error result `source: DataSource::Api,` → `source: CacheSource::Api,`

Then add, at the end of the file:

```rust
/// Invalidate the events cache in both tiers (Moka + redb).
#[allow(dead_code)]
pub fn invalidate_events_cache() {
    EVENTS_CACHE.invalidate(&EVENTS_CACHE_KEY.to_string());
    db::remove(EVENTS_TABLE, EVENTS_CACHE_KEY);
}
```

(`db` and `EVENTS_TABLE` are already imported/defined in this file from the redb work.)

- [ ] **Step 4: traders.rs**

Remove `use std::fmt;`, add `use crate::services::source::CacheSource;`, delete the `DataSource` enum + `Display` impl. Change both result fields `pub source: DataSource,` → `pub source: CacheSource,` (in `TraderNamesResult` and `TraderItemsResult`).

In `get_trader_names`, replace by branch meaning:
- L1 Moka hit: `source: DataSource::Cache` → `source: CacheSource::Memory`
- fresh redb names: `source: DataSource::Cache` → `source: CacheSource::Disk`
- API success: `source: DataSource::Api` → `source: CacheSource::Api`
- stale redb names: `source: DataSource::Cache` → `source: CacheSource::Disk`
- hardcoded fallback: `source: DataSource::Fallback` → `source: CacheSource::Fallback`

In `get_trader_items`:
- L1 Moka hit: `source: DataSource::Cache` → `source: CacheSource::Memory`
- fresh redb items: `source: DataSource::Cache` → `source: CacheSource::Disk`
- re-check-Moka block:
  ```rust
  source: if fetch_error.is_none() {
      DataSource::Api
  } else {
      DataSource::Cache
  },
  ```
  →
  ```rust
  source: if fetch_error.is_none() {
      CacheSource::Api
  } else {
      CacheSource::Memory
  },
  ```
- stale redb items: `source: DataSource::Cache` → `source: CacheSource::Disk`
- nothing-anywhere fallback: `source: DataSource::Fallback` → `source: CacheSource::Fallback`

- [ ] **Step 5: Verify build + tests**

Run: `cargo test --no-default-features --features desktop`
Expected: PASS — compiles (views still build via `CacheSource: Display`); `services::source::tests` + all prior tests pass. (The materials/trader banner *colors* may be temporarily off because the old `text.contains("Cache")` no longer matches "Memory"/"Disk" — purely cosmetic, fixed in Task 4.)

- [ ] **Step 6: Commit**

```bash
git add src/services/items.rs src/services/bots.rs src/services/events.rs src/services/traders.rs
git commit -m "feat(cache): map services to tier-aware CacheSource; add invalidate_events_cache"
```

---

### Task 4: Render `CacheBadge` on all four data pages

Refactor the two existing banners and add the badge to the two pages that lacked it. Each view captures `source`/`count`/`error` from the service result into signals and renders `CacheBadge` when not loading.

**Files:** Modify `src/components/items_view.rs`, `src/components/trader_view.rs`, `src/components/arcs_view.rs`, `src/components/events_view.rs`, and the corresponding `assets/styling/*.css`.

- [ ] **Step 1: items_view.rs**

Add imports: `use crate::components::CacheBadge;` (or extend the existing `use super::{…}` to include `CacheBadge`) and `use crate::services::source::CacheSource;`.

Replace the loading/debug signals:
```rust
    let mut is_loading = use_signal(|| true);
    let mut debug_info = use_signal(|| String::from("Fetching items..."));
```
with:
```rust
    let mut is_loading = use_signal(|| true);
    let mut data_source = use_signal(|| CacheSource::Api);
    let mut data_count = use_signal(|| 0usize);
    let mut data_error: Signal<Option<String>> = use_signal(|| None);
```

Replace the resource body:
```rust
    let all_items = use_resource(move || async move {
        is_loading.set(true);
        debug_info.set("Fetching items...".to_string());

        let result = get_all_items().await;

        let mut debug = format!("Source: {} | Count: {}", result.source, result.count);
        if let Some(ref err) = result.error {
            debug.push_str(&format!(" | Error: {}", err));
        }
        debug_info.set(debug);
        is_loading.set(false);

        result.items
    });
```
with:
```rust
    let all_items = use_resource(move || async move {
        is_loading.set(true);
        let result = get_all_items().await;
        data_source.set(result.source);
        data_count.set(result.count);
        data_error.set(result.error.clone());
        is_loading.set(false);
        result.items
    });
```

Delete the banner-class block:
```rust
    // Debug banner class
    let debug_text = debug_info();
    let banner_class = if debug_text.contains("Source: API") {
        "items-debug-banner items-debug-banner--api"
    } else if debug_text.contains("Source: Cache") {
        "items-debug-banner items-debug-banner--cache"
    } else {
        "items-debug-banner items-debug-banner--error"
    };
```

Replace the rendered banner:
```rust
            // Debug banner
            div {
                class: "items-debug",
                div {
                    class: "{banner_class}",
                    "{debug_text}"
                }
            }
```
with:
```rust
            // Cache source badge
            if !loading {
                div {
                    class: "items-debug",
                    CacheBadge {
                        source: data_source(),
                        count: Some(data_count()),
                        error: data_error(),
                    }
                }
            }
```

- [ ] **Step 2: items_view.css cleanup**

In `assets/styling/items_view.css`, delete the now-dead rule blocks: `.items-debug-banner`, `.items-debug-banner--api`, `.items-debug-banner--cache`, `.items-debug-banner--error`, and their `:root.light` variants. Keep `.items-debug` (the layout wrapper) and the `@keyframes items-fade-in` if referenced elsewhere.

- [ ] **Step 3: trader_view.rs**

Add imports: extend `use super::{Dropdown, Spinner, TraderItemCard};` to also import `CacheBadge`, and add `use crate::services::source::CacheSource;`.

Replace the debug signals:
```rust
    let mut names_source = use_signal(|| String::from("Pending..."));
    let mut items_debug = use_signal(|| String::from("Pending..."));
```
with:
```rust
    let mut names_source = use_signal(|| CacheSource::Fallback);
    let mut names_error: Signal<Option<String>> = use_signal(|| None);
    let mut items_source = use_signal(|| CacheSource::Memory);
    let mut items_count = use_signal(|| 0usize);
    let mut items_error: Signal<Option<String>> = use_signal(|| None);
```

In the `trader_names` resource, replace:
```rust
        let mut debug = format!("Names source: {}", result.source);
        if let Some(ref err) = result.error {
            debug.push_str(&format!(" | Error: {}", err));
        }
        names_source.set(debug);
```
with:
```rust
        names_source.set(result.source);
        names_error.set(result.error.clone());
```

In the `trader_items` resource, replace:
```rust
        let mut debug = format!("Items source: {} | Count: {}", result.source, result.count);
        if let Some(ref err) = result.error {
            debug.push_str(&format!(" | Error: {}", err));
        }
        items_debug.set(debug);
```
with:
```rust
        items_source.set(result.source);
        items_count.set(result.count);
        items_error.set(result.error.clone());
```

Delete both `*_banner_class` blocks (the `names_source_text`/`items_debug_text` `contains(...)` logic).

Replace the rendered banners:
```rust
            div {
                class: "trader-debug",
                div {
                    class: "{names_banner_class}",
                    "📡 {names_source_text}"
                }
                div {
                    class: "{items_banner_class}",
                    "📦 {items_debug_text}"
                }
            }
```
with:
```rust
            div {
                class: "trader-debug",
                CacheBadge {
                    source: names_source(),
                    label: Some("Names".to_string()),
                    error: names_error(),
                }
                CacheBadge {
                    source: items_source(),
                    count: Some(items_count()),
                    label: Some("Items".to_string()),
                    error: items_error(),
                }
            }
```

- [ ] **Step 4: trader_view.css cleanup**

In `assets/styling/trader_view.css`, delete the dead rule blocks: `.trader-debug-banner`, `.trader-debug-banner--api`, `.trader-debug-banner--cache`, `.trader-debug-banner--fallback`, and their `:root.light` variants. Keep `.trader-debug` (layout wrapper).

- [ ] **Step 5: arcs_view.rs**

Add imports: extend `use super::{ArcCard, Spinner};` to include `CacheBadge`; add `use crate::services::source::CacheSource;`.

Replace the loading signal + resource:
```rust
    let mut is_loading = use_signal(|| true);

    let bots_res = use_resource(move || async move {
        is_loading.set(true);
        let result = get_all_bots().await;
        is_loading.set(false);
        result.bots
    });
```
with:
```rust
    let mut is_loading = use_signal(|| true);
    let mut data_source = use_signal(|| CacheSource::Api);
    let mut data_count = use_signal(|| 0usize);
    let mut data_error: Signal<Option<String>> = use_signal(|| None);

    let bots_res = use_resource(move || async move {
        is_loading.set(true);
        let result = get_all_bots().await;
        data_source.set(result.source);
        data_count.set(result.count);
        data_error.set(result.error.clone());
        is_loading.set(false);
        result.bots
    });
```

In the rendered `div { class: "arcs-view", … }`, insert the badge right after the `arcs-view__controls` block and before the `if loading { … }`:
```rust
            if !loading {
                div { class: "arcs-view__badge",
                    CacheBadge {
                        source: data_source(),
                        count: Some(data_count()),
                        error: data_error(),
                    }
                }
            }
```

- [ ] **Step 6: arcs_view.css — add wrapper**

Append to `assets/styling/arcs_view.css`:
```css
.arcs-view__badge {
    width: 100%;
    padding: 2vw 0;
    display: flex;
    justify-content: center;
}
```

- [ ] **Step 7: events_view.rs**

Add imports: extend `use super::{EventCard, EventFilters, Spinner};` to include `CacheBadge`; add `use crate::services::source::CacheSource;`.

Replace the resource:
```rust
    let events_res = use_resource(move || async move {
        let _ = refresh(); // subscribe: re-runs the fetch whenever `refresh` changes
        get_event_schedule().await.events
    });
```
with:
```rust
    let mut data_source = use_signal(|| CacheSource::Api);
    let mut data_count = use_signal(|| 0usize);
    let mut data_error: Signal<Option<String>> = use_signal(|| None);

    let events_res = use_resource(move || async move {
        let _ = refresh(); // subscribe: re-runs the fetch whenever `refresh` changes
        let result = get_event_schedule().await;
        data_source.set(result.source);
        data_count.set(result.count);
        data_error.set(result.error.clone());
        result.events
    });
```

In the render, inside the `} else {` arm (not loading), as the first child before the `if !all.is_empty() { EventFilters { … } }`:
```rust
                div { class: "events-view__badge",
                    CacheBadge {
                        source: data_source(),
                        count: Some(data_count()),
                        error: data_error(),
                    }
                }
```

- [ ] **Step 8: events_view.css — add wrapper**

Append to `assets/styling/events_view.css`:
```css
.events-view__badge {
    width: 100%;
    padding: 2vw 0;
    display: flex;
    justify-content: center;
}
```

- [ ] **Step 9: Verify build + lint**

Run: `cargo test --no-default-features --features desktop`
Expected: PASS — compiles and all tests pass.
Run: `cargo clippy --no-default-features --features desktop`
Expected: no new warnings in the four view files or `cache_badge.rs` (pre-existing warnings elsewhere are out of scope).

- [ ] **Step 10: Commit**

```bash
git add src/components/items_view.rs src/components/trader_view.rs src/components/arcs_view.rs src/components/events_view.rs \
        assets/styling/items_view.css assets/styling/trader_view.css assets/styling/arcs_view.css assets/styling/events_view.css
git commit -m "feat(cache): render tier-aware CacheBadge on all four data pages"
```

---

### Task 5: Settings → Advanced → Clear cache

**Files:**
- Modify: `src/views/settings.rs`
- Create: `assets/styling/settings.css`

- [ ] **Step 1: Create the CSS**

Create `assets/styling/settings.css`:

```css
.settings-advanced {
    margin-top: 1.5rem;
    padding-top: 1rem;
    border-top: 1px solid rgba(255, 255, 255, 0.12);
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
}
:root.light .settings-advanced {
    border-top-color: rgba(0, 0, 0, 0.12);
}
.settings-advanced__title {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.7;
}
.settings-advanced__clear-btn {
    align-self: flex-start;
    padding: 0.5rem 1rem;
    border-radius: 0.4rem;
    border: 1px solid rgba(245, 158, 11, 0.4);
    background: rgba(245, 158, 11, 0.12);
    color: #fbbf24;
    font-size: 0.85rem;
    cursor: pointer;
}
.settings-advanced__clear-btn:active {
    transform: scale(0.98);
}
:root.light .settings-advanced__clear-btn {
    color: #b45309;
}
.settings-advanced__status {
    font-size: 0.8rem;
    color: #4ade80;
    animation: settings-fade-in 0.2s ease both;
}
:root.light .settings-advanced__status {
    color: #16a34a;
}
@keyframes settings-fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
}
```

- [ ] **Step 2: Wire the Advanced section**

In `src/views/settings.rs`, add these imports below the existing `use` lines:

```rust
use crate::services::bots::invalidate_bots_cache;
use crate::services::events::invalidate_events_cache;
use crate::services::items::invalidate_items_cache;
use crate::services::traders::invalidate_trader_cache;
use std::time::Duration;
```

Add the CSS asset constant after the imports:

```rust
const SETTINGS_CSS: Asset = asset!("/assets/styling/settings.css");
```

Inside `Settings`, add a signal at the top of the function body (after `let is_dark = dark_mode();`):

```rust
    let mut cache_cleared = use_signal(|| false);
```

In the `rsx!`, add the stylesheet link as the first child and the Advanced section as the last child inside `PageLayout`:

```rust
    rsx! {
        document::Link { rel: "stylesheet", href: SETTINGS_CSS }
        PageLayout {
            title: "Settings",
            // ... existing Toggle and Dropdown unchanged ...

            div { class: "settings-advanced",
                div { class: "settings-advanced__title", "Advanced" }
                button {
                    class: "settings-advanced__clear-btn",
                    onclick: move |_| {
                        invalidate_items_cache();
                        invalidate_bots_cache();
                        invalidate_events_cache();
                        invalidate_trader_cache();
                        cache_cleared.set(true);
                        spawn(async move {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            cache_cleared.set(false);
                        });
                    },
                    "Clear cache"
                }
                if cache_cleared() {
                    div { class: "settings-advanced__status", "Cache cleared" }
                }
            }
        }
    }
```

(Keep the existing `Toggle`, `Dropdown`, and `use_effect` exactly as they are; only add the stylesheet link and the `settings-advanced` block.)

- [ ] **Step 3: Verify build + lint**

Run: `cargo test --no-default-features --features desktop`
Expected: PASS.
Run: `cargo clippy --no-default-features --features desktop`
Expected: no new warnings in `settings.rs`.

- [ ] **Step 4: Manual acceptance run (best-effort, Android emulator)**

The app's real target is Android (`dx serve --platform android`; see `[[project-android-deploy-and-cache-path]]`). If a device/emulator and `dx` are available, rebuild/redeploy and verify:
1. Each data page (Materials, Traders, Arcs, Events) shows a cache badge.
2. On a cold load it reads **API**; revisiting within the TTL reads **Memory**; after an app restart (Moka empty, redb warm) it reads **Disk**.
3. Settings → Advanced → **Clear cache** shows "Cache cleared" for ~2s, and the next page visit reads **API** again.

If no device is available, report this step as SKIPPED — Steps 3's `cargo test`/`clippy` are the hard gates.

- [ ] **Step 5: Commit**

```bash
git add src/views/settings.rs assets/styling/settings.css
git commit -m "feat(settings): add Advanced section with Clear cache (Moka + redb)"
```

---

## Self-Review Notes

**Spec coverage:**
- Shared `CacheSource` (API/Memory/Disk/Fallback) + label/css_class → Task 1. ✓
- Reusable `CacheBadge` + CSS → Task 2. ✓
- Per-service tier mapping (Moka→Memory, fresh/stale redb→Disk, API→Api, traders→Fallback) → Task 3. ✓
- `invalidate_events_cache` → Task 3 Step 3. ✓
- Badge on items + traders (refactor) and arcs + events (add) → Task 4. ✓
- Drop dead `*-debug-banner` CSS → Task 4 Steps 2, 4. ✓
- Settings Advanced + one-tap clear + inline confirmation (~2s) clearing both tiers → Task 5. ✓

**Type consistency:** `CacheSource` variants `Api/Memory/Disk/Fallback` used identically across Tasks 1–4; `CacheBadge` prop names (`source`, `count`, `label`, `error`) match every call site; `Option<usize>` count passed as `Some(...)`, `Option<String>` error/label passed directly or as `Some(...)`.

**Decisions honored:** fresh and stale redb both map to `Disk` (no Stale state); shared enum + reusable component (not per-service); one-tap clear with inline confirmation (no modal); trader 📡/📦 emoji dropped for a unified badge.

**Notes:** `CacheSource` keeps a `Display` impl so Task 3 leaves the views compiling before Task 4 migrates them (a brief, harmless banner-color mismatch exists only between Task 3 and Task 4). `tokio::time::sleep` is already used in `events_view.rs`, so the `time` feature is available without a `Cargo.toml` change.
