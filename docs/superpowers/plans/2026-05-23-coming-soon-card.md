# Coming Soon Card Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a reusable centered "Coming Soon" notice card (pale-yellow backdrop, dark text) to the Map and Raider pages.

**Architecture:** A single presentational `ComingSoon` component following the codebase's established pattern (component file in `src/components/` + paired CSS in `assets/styling/`, linked via the `asset!` macro). Each page passes its own icon. The pale yellow is self-contained in the card's CSS — no global theme tokens are added. The only non-trivial logic, subtitle defaulting, is extracted into a pure `resolve_subtitle` helper and unit-tested (mirroring the existing `format_remaining` / `name_matches` test pattern).

**Tech Stack:** Rust, Dioxus 0.7 (`rsx!`, `#[component]`), manganis `asset!`, CSS with `--color-*` variables (the card overrides with literal colors).

**Spec:** `docs/superpowers/specs/2026-05-23-coming-soon-card-design.md`

---

### Task 1: Scaffold module and TDD the `resolve_subtitle` helper

**Files:**
- Create: `src/components/coming_soon.rs`
- Modify: `src/components/mod.rs`
- Test: `src/components/coming_soon.rs` (inline `#[cfg(test)] mod tests`, matching `event_card.rs`)

- [ ] **Step 1: Create the module with a failing stub + tests**

Create `src/components/coming_soon.rs` with the helper stubbed to the wrong value so the tests compile and fail on assertion:

```rust
use dioxus::prelude::*;

const DEFAULT_SUBTITLE: &str = "This feature is in development.";

/// Returns the subtitle to display: the provided text, or the default copy when
/// `subtitle` is `None` or blank.
fn resolve_subtitle(subtitle: &Option<String>) -> String {
    let _ = subtitle; // Stub — replaced in Step 3.
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_uses_default_copy() {
        assert_eq!(resolve_subtitle(&None), "This feature is in development.");
    }

    #[test]
    fn blank_falls_back_to_default() {
        assert_eq!(
            resolve_subtitle(&Some("   ".to_string())),
            "This feature is in development."
        );
    }

    #[test]
    fn custom_subtitle_is_used() {
        assert_eq!(
            resolve_subtitle(&Some("Map tools soon".to_string())),
            "Map tools soon"
        );
    }
}
```

Register the module in `src/components/mod.rs`. Add these two lines next to the other `mod`/`pub use` pairs (e.g. after the `page_layout` block):

```rust
mod coming_soon;
pub use coming_soon::ComingSoon;
```

> Note: `pub use coming_soon::ComingSoon;` will not compile yet because `ComingSoon` does not exist. To keep this task's test runnable, add a temporary minimal component now and flesh it out in Task 2:
>
> ```rust
> #[component]
> pub fn ComingSoon(icon: String, subtitle: Option<String>) -> Element {
>     let _ = (icon, subtitle);
>     rsx! {}
> }
> ```
>
> Place this `ComingSoon` stub in `coming_soon.rs` above the `resolve_subtitle` function.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test coming_soon`
Expected: the three `tests::*` tests FAIL — e.g. `none_uses_default_copy` panics with `assertion ... left: "" right: "This feature is in development."`.

- [ ] **Step 3: Implement the real helper**

Replace the stub body of `resolve_subtitle` with:

```rust
fn resolve_subtitle(subtitle: &Option<String>) -> String {
    subtitle
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SUBTITLE.to_string())
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test coming_soon`
Expected: `none_uses_default_copy`, `blank_falls_back_to_default`, `custom_subtitle_is_used` all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/coming_soon.rs src/components/mod.rs
git commit -m "feat(coming-soon): add ComingSoon module with subtitle helper"
```

---

### Task 2: Build out the `ComingSoon` component and its CSS

**Files:**
- Modify: `src/components/coming_soon.rs`
- Create: `assets/styling/coming_soon.css`

- [ ] **Step 1: Create the stylesheet**

Create `assets/styling/coming_soon.css`:

```css
.coming-soon {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
  padding-top: 12vh;
}

.coming-soon__card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  width: 100%;
  max-width: 22rem;
  padding: 2rem 1.5rem;
  text-align: center;
  background-color: #faf3c5;
  border: 0.0625rem solid #e6d98a;
  border-radius: 0.5rem;
}

.coming-soon__icon {
  width: 3rem;
  height: 3rem;
  object-fit: contain;
}

.coming-soon__title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: #2a2614;
}

.coming-soon__subtitle {
  margin: 0;
  font-size: 0.875rem;
  line-height: 1.5;
  color: #5c5530;
}
```

- [ ] **Step 2: Replace the stub component with the full implementation**

In `src/components/coming_soon.rs`, add the CSS asset constant below the existing `use dioxus::prelude::*;` line:

```rust
const COMING_SOON_CSS: Asset = asset!("/assets/styling/coming_soon.css");
```

Replace the temporary `ComingSoon` stub from Task 1 with the full component (keep `resolve_subtitle` and the `tests` module unchanged):

```rust
/// Reusable centered "Coming Soon" notice card for pages that are not yet built.
///
/// # Props
/// - `icon`: asset path for the page's icon (e.g. the Map or Raider icon).
/// - `subtitle`: optional override; `None` or blank uses the default copy.
#[component]
pub fn ComingSoon(icon: String, subtitle: Option<String>) -> Element {
    let subtitle = resolve_subtitle(&subtitle);

    rsx! {
        document::Link { rel: "stylesheet", href: COMING_SOON_CSS }
        div { class: "coming-soon",
            div { class: "coming-soon__card",
                img { class: "coming-soon__icon", src: "{icon}", alt: "Coming soon" }
                h2 { class: "coming-soon__title", "Coming Soon" }
                p { class: "coming-soon__subtitle", "{subtitle}" }
            }
        }
    }
}
```

- [ ] **Step 3: Verify it compiles and tests still pass**

Run: `cargo test coming_soon`
Expected: compiles cleanly; the three subtitle tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/coming_soon.rs assets/styling/coming_soon.css
git commit -m "feat(coming-soon): render centered pale-yellow card with icon, title, subtitle"
```

---

### Task 3: Integrate `ComingSoon` into the Map and Raider pages

**Files:**
- Modify: `src/views/map.rs`
- Modify: `src/views/raider.rs`

- [ ] **Step 1: Render `ComingSoon` on the Map page**

Replace the full contents of `src/views/map.rs` with:

```rust
use dioxus::prelude::*;

use crate::components::{ComingSoon, PageLayout};

const ICON_MAP: Asset = asset!("/assets/styling/media/icons/map.svg");

/// The Map page component that will be rendered when the current route is `[Route::Map]`
#[component]
pub fn Map() -> Element {
    rsx! {
        PageLayout {
            title: "Map",
            ComingSoon { icon: ICON_MAP.to_string() }
        }
    }
}
```

> `Asset` implements `Display`, so `.to_string()` yields the bundled asset path that the `icon: String` prop expects. `Asset` and `asset!` come from `dioxus::prelude::*`.

- [ ] **Step 2: Render `ComingSoon` on the Raider page**

Replace the full contents of `src/views/raider.rs` with:

```rust
use dioxus::prelude::*;

use crate::components::{ComingSoon, PageLayout};

const ICON_RAIDER: Asset = asset!("/assets/styling/media/icons/raider.svg");

/// The Raider page component that will be rendered when the current route is `[Route::Raider]`
#[component]
pub fn Raider() -> Element {
    rsx! {
        PageLayout {
            title: "Raider",
            ComingSoon { icon: ICON_RAIDER.to_string() }
        }
    }
}
```

- [ ] **Step 3: Verify the whole crate compiles**

Run: `cargo check`
Expected: compiles with no errors. (If `cargo check` reports a missing platform feature, use the project's normal check path, e.g. `dx check`, but `cargo check` is expected to succeed since the existing pages compile this way.)

- [ ] **Step 4: Commit**

```bash
git add src/views/map.rs src/views/raider.rs
git commit -m "feat(coming-soon): show Coming Soon card on Map and Raider pages"
```

---

### Task 4: Visual verification

**Files:** none (manual check)

- [ ] **Step 1: Run the app**

Run: `dx serve` (or `dx serve --platform android` for the on-device target).

- [ ] **Step 2: Confirm both pages**

Navigate to the **Map** page and the **Raider** page. On each, verify:
- A centered pale-yellow card appears with the page's own icon, the heading "Coming Soon", and the subtitle "This feature is in development."
- Dark text is clearly legible on the pale-yellow background.

- [ ] **Step 3: Confirm both themes**

Toggle light/dark mode (via Settings). The card stays pale yellow with dark text and remains legible in both themes.

---

## Notes for the implementer

- Follow existing conventions: components are `snake_case` files exported from `src/components/mod.rs`; each pairs with a `kebab/snake`-named CSS file in `assets/styling/` linked through `document::Link { rel: "stylesheet", href: <CSS_ASSET> }`. See `src/components/arc_card.rs` for the canonical example.
- Do **not** add yellow tokens to `assets/styling/common.css` — the card's color is intentionally self-contained (spec "Out of Scope").
- The heading text is always "Coming Soon" (not a prop) by design.
