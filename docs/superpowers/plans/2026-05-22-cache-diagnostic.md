# Dev-Only Cache-State Diagnostic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the served-only `CacheBadge` on each data page with a dev-only diagnostic showing the served tier plus the pre-load L1 (Moka) and L2 (redb) state for that key.

**Architecture:** Add `L1State`/`L2State`/`CacheState` types and a lightweight `db::l2_state` probe; add a read-only `*_cache_state()` probe per service (Moka `contains_key` + `db::l2_state`, no cascade changes); add a `CacheDiagnostic` component composing the existing `CacheBadge` plus two state pills; each view, only under `cfg!(debug_assertions)`, probes just before its fetch and renders the diagnostic.

**Tech Stack:** Rust, Dioxus 0.7 (`#[component]`, `cfg!(debug_assertions)`), `moka::sync` (`contains_key`), the existing `services::db` redb layer + `CacheEnvelope`.

**Reference spec:** `docs/superpowers/specs/2026-05-22-cache-diagnostic-design.md`

**Branch:** This continues on the existing `feat/cache-indicator` branch (the prior CacheBadge work is here, unmerged).

**Canonical commands** (default `mobile` feature won't link on a host):
- Test: `cargo test --no-default-features --features desktop`
- Lint: `cargo clippy --no-default-features --features desktop`

---

## File Structure

- **Modify** `src/services/source.rs` — add `L1State`, `L2State`, `CacheState` (+ `label()`/`css_class()`).
- **Modify** `src/services/db.rs` — add `l2_state` (and internal `l2_state_in`) + `CachedAtProbe`.
- **Modify** `src/services/{items,bots,events,traders}.rs` — add read-only `*_cache_state()` probe fn(s).
- **Create** `src/components/cache_diagnostic.rs` — the `CacheDiagnostic` component.
- **Modify** `src/components/mod.rs` — export `CacheDiagnostic`.
- **Modify** `assets/styling/cache_badge.css` — add `.cache-diagnostic` + `.cache-pill--*` styles.
- **Modify** `src/components/{items_view,trader_view,arcs_view,events_view}.rs` — probe + render diagnostic, dev-gated.

`CacheBadge` is unchanged (reused inside `CacheDiagnostic`).

---

### Task 1: L1State / L2State / CacheState (TDD)

**Files:** Modify `src/services/source.rs` (append types + tests).

- [ ] **Step 1: Write the failing tests.** In `src/services/source.rs`, inside the existing `#[cfg(test)] mod tests`, add:

```rust
    #[test]
    fn l1_state_labels_and_css() {
        assert_eq!(L1State::Hit.label(), "hit");
        assert_eq!(L1State::Miss.label(), "miss");
        assert_eq!(L1State::Hit.css_class(), "hit");
    }

    #[test]
    fn l2_state_labels_and_css() {
        assert_eq!(L2State::Fresh.label(), "fresh");
        assert_eq!(L2State::Stale.label(), "stale");
        assert_eq!(L2State::Miss.label(), "miss");
        assert_eq!(L2State::Stale.css_class(), "stale");
    }
```

- [ ] **Step 2: Run to verify failure.**
Run: `cargo test --no-default-features --features desktop services::source`
Expected: FAIL — `cannot find type L1State`/`L2State`.

- [ ] **Step 3: Add the types.** In `src/services/source.rs`, before the `#[cfg(test)] mod tests` line, append:

```rust
/// State of the Moka (L1) in-memory cache for a given key, at probe time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L1State {
    Hit,
    Miss,
}

impl L1State {
    /// Lowercase label for the diagnostic pill text.
    pub fn label(&self) -> &'static str {
        match self {
            L1State::Hit => "hit",
            L1State::Miss => "miss",
        }
    }

    /// CSS modifier suffix for the `cache-pill--{}` class (same string as the label).
    pub fn css_class(&self) -> &'static str {
        self.label()
    }
}

/// State of the redb (L2) on-disk cache for a given key, at probe time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L2State {
    Fresh,
    Stale,
    Miss,
}

impl L2State {
    /// Lowercase label for the diagnostic pill text.
    pub fn label(&self) -> &'static str {
        match self {
            L2State::Fresh => "fresh",
            L2State::Stale => "stale",
            L2State::Miss => "miss",
        }
    }

    /// CSS modifier suffix for the `cache-pill--{}` class (same string as the label).
    pub fn css_class(&self) -> &'static str {
        self.label()
    }
}

/// The pre-load state of both cache tiers for a key, shown by the dev diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheState {
    pub l1: L1State,
    pub l2: L2State,
}
```

- [ ] **Step 4: Run to verify pass.**
Run: `cargo test --no-default-features --features desktop services::source`
Expected: PASS — the two new tests plus the existing `CacheSource` tests.

- [ ] **Step 5: Commit.**
```bash
git add src/services/source.rs
git commit -m "feat(cache): add L1State/L2State/CacheState diagnostic types"
```
(End every commit message with: `Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>`)

---

### Task 2: `db::l2_state` redb freshness probe (TDD)

**Files:** Modify `src/services/db.rs`.

- [ ] **Step 1: Write the failing test.** In `src/services/db.rs`, inside `#[cfg(test)] mod tests`, add:

```rust
    #[test]
    fn l2_state_reports_fresh_stale_and_miss() {
        let (_dir, db) = temp_db();

        // Fresh: written now, within TTL.
        write_in(&db, TT, "fresh", &vec![1u8, 2, 3]);
        assert_eq!(l2_state_in(&db, TT, "fresh", Duration::from_secs(900)), L2State::Fresh);

        // Stale: craft an envelope timestamped 10000s in the past.
        let env = CacheEnvelope { cached_at: now_secs() - 10_000, data: vec![9u8] };
        let bytes = serde_json::to_vec(&env).unwrap();
        let txn = db.begin_write().unwrap();
        {
            let mut t = txn.open_table(TT).unwrap();
            t.insert("stale", bytes.as_slice()).unwrap();
        }
        txn.commit().unwrap();
        assert_eq!(l2_state_in(&db, TT, "stale", Duration::from_secs(900)), L2State::Stale);

        // Miss: absent key.
        assert_eq!(l2_state_in(&db, TT, "absent", Duration::from_secs(900)), L2State::Miss);
    }
```

- [ ] **Step 2: Run to verify failure.**
Run: `cargo test --no-default-features --features desktop services::db`
Expected: FAIL — `cannot find function l2_state_in` and `L2State` not in scope.

- [ ] **Step 3: Implement.** In `src/services/db.rs`, add `use crate::services::source::L2State;` to the imports. Then add, after the `write_in` function (before the `#[cfg(test)]` module):

```rust
/// Minimal view of an envelope used by `l2_state` to read just the timestamp,
/// ignoring the (possibly large) `data` field (serde skips unknown fields).
#[derive(serde::Deserialize)]
struct CachedAtProbe {
    cached_at: i64,
}

/// Classify the redb (L2) state for `key` without deserializing the payload.
fn l2_state_in(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
    ttl: Duration,
) -> L2State {
    let cached_at = (|| -> Option<i64> {
        let txn = db.begin_read().ok()?;
        let tbl = txn.open_table(table).ok()?;
        let guard = tbl.get(key).ok()??;
        let probe: CachedAtProbe = serde_json::from_slice(guard.value()).ok()?;
        Some(probe.cached_at)
    })();

    match cached_at {
        Some(ts) => {
            let age = now_secs().saturating_sub(ts);
            if age >= 0 && (age as u64) <= ttl.as_secs() {
                L2State::Fresh
            } else {
                L2State::Stale
            }
        }
        None => L2State::Miss,
    }
}

/// Dev-diagnostic probe of the redb (L2) state for `key`. Returns `Miss` if the
/// cache is unavailable. Read-only; does not touch Moka.
pub fn l2_state(table: TableDefinition<&str, &[u8]>, key: &str, ttl: Duration) -> L2State {
    match DB.as_ref() {
        Some(db) => l2_state_in(db, table, key, ttl),
        None => L2State::Miss,
    }
}
```

- [ ] **Step 4: Run to verify pass.**
Run: `cargo test --no-default-features --features desktop services::db`
Expected: PASS — including `l2_state_reports_fresh_stale_and_miss`.

- [ ] **Step 5: Commit.**
```bash
git add src/services/db.rs
git commit -m "feat(cache): add db::l2_state redb freshness probe"
```

---

### Task 3: Per-service read-only cache-state probes

Each probe reads Moka (`contains_key`, non-mutating) + `db::l2_state`. No cascade changes. `Duration`, `db`, the cache statics, keys, tables, and `CACHE_TTL_SECS` are already in scope in each file from prior work.

**Files:** Modify `src/services/items.rs`, `bots.rs`, `events.rs`, `traders.rs`.

- [ ] **Step 1: items.rs.** Extend the existing `use crate::services::source::...` line to also import `CacheState` and `L1State` (it already imports `CacheSource`). Add at the end of the file:

```rust
/// Dev-diagnostic: read-only probe of the L1/L2 state for the items key.
pub fn items_cache_state() -> crate::services::source::CacheState {
    let l1 = if ITEMS_CACHE.contains_key(ITEMS_CACHE_KEY) {
        L1State::Hit
    } else {
        L1State::Miss
    };
    let l2 = db::l2_state(ITEMS_TABLE, ITEMS_CACHE_KEY, Duration::from_secs(CACHE_TTL_SECS));
    CacheState { l1, l2 }
}
```

- [ ] **Step 2: bots.rs.** Extend the `use crate::services::source::...` import to add `CacheState, L1State`. Add at end of file:

```rust
/// Dev-diagnostic: read-only probe of the L1/L2 state for the bots key.
pub fn bots_cache_state() -> CacheState {
    let l1 = if BOTS_CACHE.contains_key(BOTS_CACHE_KEY) {
        L1State::Hit
    } else {
        L1State::Miss
    };
    let l2 = db::l2_state(BOTS_TABLE, BOTS_CACHE_KEY, Duration::from_secs(CACHE_TTL_SECS));
    CacheState { l1, l2 }
}
```

- [ ] **Step 3: events.rs.** Extend the `use crate::services::source::...` import to add `CacheState, L1State`. Add at end of file:

```rust
/// Dev-diagnostic: read-only probe of the L1/L2 state for the events key.
pub fn events_cache_state() -> CacheState {
    let l1 = if EVENTS_CACHE.contains_key(EVENTS_CACHE_KEY) {
        L1State::Hit
    } else {
        L1State::Miss
    };
    let l2 = db::l2_state(EVENTS_TABLE, EVENTS_CACHE_KEY, Duration::from_secs(CACHE_TTL_SECS));
    CacheState { l1, l2 }
}
```

- [ ] **Step 4: traders.rs.** Extend the `use crate::services::source::...` import to add `CacheState, L1State`. Add at end of file:

```rust
/// Dev-diagnostic: read-only probe of the L1/L2 state for the trader-names key.
pub fn trader_names_cache_state() -> CacheState {
    let l1 = if TRADERS_NAME_CACHE.contains_key(TRADER_NAMES_KEY) {
        L1State::Hit
    } else {
        L1State::Miss
    };
    let l2 = db::l2_state(TRADER_NAMES_TABLE, TRADER_NAMES_KEY, Duration::from_secs(CACHE_TTL_SECS));
    CacheState { l1, l2 }
}

/// Dev-diagnostic: read-only probe of the L1/L2 state for a trader's items.
/// Moka key is prefixed; the redb key is the bare trader name (matching the cascade).
pub fn trader_items_cache_state(name: &str) -> CacheState {
    let cache_key = format!("{}{}", TRADER_ITEMS_PREFIX, name);
    let l1 = if TRADERS_ITEMS_CACHE.contains_key(&cache_key) {
        L1State::Hit
    } else {
        L1State::Miss
    };
    let l2 = db::l2_state(TRADER_ITEMS_TABLE, name, Duration::from_secs(CACHE_TTL_SECS));
    CacheState { l1, l2 }
}
```

- [ ] **Step 5: Verify build.**
Run: `cargo build --no-default-features --features desktop`
Expected: `Finished`. The probe fns are `pub` and not yet called → a "never used" dead-code warning is expected here and clears in Task 5. If `contains_key` raises a trait-bound error on the `&str`/`String` key, pass `&KEY.to_string()` instead (e.g. `BOTS_CACHE.contains_key(&BOTS_CACHE_KEY.to_string())`) and note it.

- [ ] **Step 6: Commit.**
```bash
git add src/services/items.rs src/services/bots.rs src/services/events.rs src/services/traders.rs
git commit -m "feat(cache): add read-only per-service L1/L2 cache-state probes"
```

---

### Task 4: `CacheDiagnostic` component + pill styles

**Files:** Create `src/components/cache_diagnostic.rs`; modify `src/components/mod.rs`, `assets/styling/cache_badge.css`.

- [ ] **Step 1: Create the component.** `src/components/cache_diagnostic.rs`:

```rust
use dioxus::prelude::*;

use super::CacheBadge;
use crate::services::source::{CacheSource, CacheState};

/// Dev-only diagnostic row: the served-tier `CacheBadge` plus L1 (Moka) and
/// L2 (redb) state pills, showing the pre-load state of both caches for a key.
#[component]
pub fn CacheDiagnostic(
    source: CacheSource,
    state: CacheState,
    #[props(default)] count: Option<usize>,
    #[props(default)] label: Option<String>,
    #[props(default)] error: Option<String>,
) -> Element {
    let l1_class = format!("cache-pill cache-pill--{}", state.l1.css_class());
    let l2_class = format!("cache-pill cache-pill--{}", state.l2.css_class());

    rsx! {
        div {
            class: "cache-diagnostic",
            CacheBadge { source, count, label, error }
            span { class: "{l1_class}", "L1: {state.l1.label()}" }
            span { class: "{l2_class}", "L2: {state.l2.label()}" }
        }
    }
}
```

(The `.cache-pill` styles live in `cache_badge.css`, which the embedded `CacheBadge` already links via `document::Link`, so no extra link is needed.)

- [ ] **Step 2: Export it.** In `src/components/mod.rs`, after the `cache_badge` block, add:

```rust
mod cache_diagnostic;
pub use cache_diagnostic::CacheDiagnostic;
```

- [ ] **Step 3: Add pill styles.** Append to `assets/styling/cache_badge.css`:

```css
/* --- Dev cache diagnostic: served chip + L1/L2 state pills --- */
.cache-diagnostic {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
}

.cache-pill {
    font-size: 0.7rem;
    font-family: monospace;
    padding: 0.35rem 0.6rem;
    border-radius: 0.35rem;
    border: 1px solid transparent;
}

.cache-pill--hit {
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
    border-color: rgba(34, 197, 94, 0.3);
}
.cache-pill--fresh {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
    border-color: rgba(59, 130, 246, 0.3);
}
.cache-pill--stale {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
    border-color: rgba(245, 158, 11, 0.3);
}
.cache-pill--miss {
    background: rgba(148, 163, 184, 0.15);
    color: #94a3b8;
    border-color: rgba(148, 163, 184, 0.3);
}

:root.light .cache-pill--hit { color: #16a34a; }
:root.light .cache-pill--fresh { color: #2563eb; }
:root.light .cache-pill--stale { color: #b45309; }
:root.light .cache-pill--miss { color: #64748b; }
```

- [ ] **Step 4: Verify build.**
Run: `cargo check --no-default-features --features desktop`
Expected: `Finished` (a `CacheDiagnostic` "never used" warning is fine until Task 5).

- [ ] **Step 5: Commit.**
```bash
git add src/components/cache_diagnostic.rs src/components/mod.rs assets/styling/cache_badge.css
git commit -m "feat(cache): add CacheDiagnostic component with L1/L2 state pills"
```

---

### Task 5: Wire the diagnostic into all four views (dev-gated)

Each view: import `CacheDiagnostic` (replacing the now-internal `CacheBadge` import), `CacheState`, and the probe fn; add a `cache_state` signal; probe just before the fetch under `cfg!(debug_assertions)`; render `CacheDiagnostic` (replacing `CacheBadge`) under `cfg!(debug_assertions)`. Read each file before editing.

**Files:** Modify `src/components/items_view.rs`, `trader_view.rs`, `arcs_view.rs`, `events_view.rs`.

- [ ] **Step 1: items_view.rs.**
Imports: change the `CacheBadge` import to `CacheDiagnostic`; add `use crate::services::source::CacheState;` (keep the existing `CacheSource` import); add `items_cache_state` to the items service import (e.g. `use crate::services::items::{get_all_items, items_cache_state};`).

Add a signal alongside the others (after `data_error`):
```rust
    let mut cache_state: Signal<Option<CacheState>> = use_signal(|| None);
```

In the `all_items` resource, add the probe right after `is_loading.set(true);` and before `let result = get_all_items().await;`:
```rust
        if cfg!(debug_assertions) {
            cache_state.set(Some(items_cache_state()));
        }
```

Replace the rendered badge block:
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
with:
```rust
            // Cache diagnostic (dev builds only)
            if !loading && cfg!(debug_assertions) {
                if let Some(state) = cache_state() {
                    div {
                        class: "items-debug",
                        CacheDiagnostic {
                            source: data_source(),
                            count: Some(data_count()),
                            error: data_error(),
                            state,
                        }
                    }
                }
            }
```

- [ ] **Step 2: arcs_view.rs.**
Imports: change `CacheBadge` → `CacheDiagnostic`; add `use crate::services::source::CacheState;` (keep `CacheSource`); add `bots_cache_state` to the bots import (e.g. `use crate::services::bots::{get_all_bots, bots_cache_state};`).

Add signal after `data_error`:
```rust
    let mut cache_state: Signal<Option<CacheState>> = use_signal(|| None);
```

In the `bots_res` resource, add right after `is_loading.set(true);`:
```rust
        if cfg!(debug_assertions) {
            cache_state.set(Some(bots_cache_state()));
        }
```

Replace the badge block:
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
with:
```rust
            if !loading && cfg!(debug_assertions) {
                if let Some(state) = cache_state() {
                    div { class: "arcs-view__badge",
                        CacheDiagnostic {
                            source: data_source(),
                            count: Some(data_count()),
                            error: data_error(),
                            state,
                        }
                    }
                }
            }
```

- [ ] **Step 3: events_view.rs.**
Imports: change `CacheBadge` → `CacheDiagnostic`; add `use crate::services::source::CacheState;` (keep `CacheSource`); add `events_cache_state` to the events import (e.g. `use crate::services::events::{get_event_schedule, events_cache_state};`).

Add signal alongside `data_source`/`data_count`/`data_error`:
```rust
    let mut cache_state: Signal<Option<CacheState>> = use_signal(|| None);
```

In the `events_res` resource, add the probe after `let _ = refresh();` and before `let result = get_event_schedule().await;`:
```rust
        if cfg!(debug_assertions) {
            cache_state.set(Some(events_cache_state()));
        }
```

Replace the badge block:
```rust
                div { class: "events-view__badge",
                    CacheBadge {
                        source: data_source(),
                        count: Some(data_count()),
                        error: data_error(),
                    }
                }
```
with:
```rust
                if cfg!(debug_assertions) {
                    if let Some(state) = cache_state() {
                        div { class: "events-view__badge",
                            CacheDiagnostic {
                                source: data_source(),
                                count: Some(data_count()),
                                error: data_error(),
                                state,
                            }
                        }
                    }
                }
```

- [ ] **Step 4: trader_view.rs.**
Imports: change `CacheBadge` → `CacheDiagnostic`; add `use crate::services::source::CacheState;` (keep `CacheSource`); add the two probes to the traders import (e.g. `use crate::services::traders::{get_trader_items, get_trader_names, trader_items_cache_state, trader_names_cache_state};`).

Add two signals alongside the existing source/error signals:
```rust
    let mut names_state: Signal<Option<CacheState>> = use_signal(|| None);
    let mut items_state: Signal<Option<CacheState>> = use_signal(|| None);
```

In the `trader_names` resource, before `let result = get_trader_names().await;`, add:
```rust
        if cfg!(debug_assertions) {
            names_state.set(Some(trader_names_cache_state()));
        }
```

In the `trader_items` resource, the non-empty branch calls `get_trader_items(&trader_name).await`. Immediately before that call (inside the `else`), add:
```rust
            if cfg!(debug_assertions) {
                items_state.set(Some(trader_items_cache_state(&trader_name)));
            }
```

Replace the two-badge render block:
```rust
            if !loading {
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
            }
```
with:
```rust
            if !loading && cfg!(debug_assertions) {
                div {
                    class: "trader-debug",
                    if let Some(state) = names_state() {
                        CacheDiagnostic {
                            source: names_source(),
                            label: Some("Names".to_string()),
                            error: names_error(),
                            state,
                        }
                    }
                    if let Some(state) = items_state() {
                        CacheDiagnostic {
                            source: items_source(),
                            count: Some(items_count()),
                            label: Some("Items".to_string()),
                            error: items_error(),
                            state,
                        }
                    }
                }
            }
```

- [ ] **Step 5: Verify build + tests + lint.**
Run: `cargo test --no-default-features --features desktop`
Expected: PASS — all tests; the Task-3 probe "never used" warnings are now gone (the views call them under `cfg!`).
Run: `cargo clippy --no-default-features --features desktop`
Expected: no new warnings in the four views, `cache_diagnostic.rs`, the services, `source.rs`, or `db.rs`.

- [ ] **Step 6: Manual acceptance (best-effort, Android emulator dev build).**
If `dx` + emulator are available (see `[[project-android-deploy-and-cache-path]]`), rebuild/redeploy and verify each data page shows `[Served] L1: … L2: …`: cold = `API / miss / miss`; reload = `Memory / hit / fresh`; after an app restart = `Disk / miss / fresh`; after Settings → Clear cache = `API / miss / miss`. (A `--release` build would show no diagnostic — optional to confirm.) If no device, report SKIPPED — Step 5 is the hard gate.

- [ ] **Step 7: Commit.**
```bash
git add src/components/items_view.rs src/components/trader_view.rs src/components/arcs_view.rs src/components/events_view.rs
git commit -m "feat(cache): render dev-only CacheDiagnostic (served + L1/L2) on data pages"
```

---

## Self-Review Notes

**Spec coverage:**
- `L1State`/`L2State`/`CacheState` (+label/css_class) → Task 1. ✓
- `db::l2_state` lightweight `{cached_at}`-only probe (Fresh/Stale/Miss) → Task 2. ✓
- Per-service read-only probes (Moka `contains_key` + `db::l2_state`), incl. traders names + items with the bare-name redb key → Task 3. ✓
- `CacheDiagnostic` composing `CacheBadge` + L1/L2 pills, with pill CSS → Task 4. ✓
- Views probe pre-load and render the diagnostic, all under `cfg!(debug_assertions)`; `CacheBadge` now only inside `CacheDiagnostic` → Task 5. ✓
- Dev-only gating at both probe call and render (release renders nothing, no extra reads) → Task 5 `cfg!(debug_assertions)` guards. ✓

**Type consistency:** `CacheState { l1: L1State, l2: L2State }` used identically in services (Task 3), component (Task 4), and views (Task 5); `db::l2_state(table, key, ttl) -> L2State` signature matches every probe call; `CacheDiagnostic` props (`source`, `state`, `count`, `label`, `error`) match all four view call sites (traders passes `label`, items/arcs/events pass `count`).

**Decisions honored:** served chip + two pills layout; `cfg!(debug_assertions)` runtime gate; pre-load probe semantics (probe runs before the fetch); decoupled probe leaves the cascade/`get_*()` untouched.

**Notes:** Task 3 leaves a temporary "never used" warning on the probe fns until Task 5 calls them (expected, like prior phases). `contains_key` key-type fallback noted in Task 3 Step 5 if the `&str` form trips a trait bound. The events badge stays in its `else` (not-loading) arm — gated additionally by `cfg!(debug_assertions)`.
