# Coming Soon Card — Design

**Date:** 2026-05-23
**Status:** Approved (design)

## Purpose

The **Map** and **Raider** pages are currently empty placeholders (each renders only
`PageLayout { title }`). Before building the first APK, give users a clear signal that
these pages are intentional but not yet functional, via a reusable "Coming Soon" card.

## Requirements

- A single **reusable** component used by both the Map and Raider pages.
- **Centered hero** layout: the card sits centered in the page content area, prominent
  on an otherwise empty page.
- Card content: **icon → "Coming Soon" heading → subtitle**.
  - Icon is the page's own icon (`map.svg` / `raider.svg`), passed in by the page.
  - Subtitle defaults to "This feature is in development." (overridable).
- **Pale yellow backdrop** (`#faf3c5`) with **dark text** so the notice is bold and
  reads correctly in both the dark (default) and light themes.

## Approach

Follow the established component pattern in this codebase: a component file under
`src/components/`, exported from `src/components/mod.rs`, paired with a CSS file under
`assets/styling/` linked via the `asset!` macro + `document::Link` (mirrors `arc_card`).

The pale yellow is **self-contained in `coming_soon.css`** rather than added as global
`--color-*` theme tokens. Rationale: yellow here is a deliberate "notice" color that is
theme-independent (dark text on pale yellow has strong contrast in both themes), so
scoping it to the card avoids polluting the shared palette in `common.css`. This also
keeps the change surgical.

## Component API

`src/components/coming_soon.rs`

```rust
#[component]
pub fn ComingSoon(
    icon: String,             // asset path for the page icon, e.g. map.svg
    subtitle: Option<String>, // defaults to "This feature is in development."
) -> Element
```

Renders a centered wrapper containing the card:

- icon (`<img>`, ~3rem)
- `h2` "Coming Soon"
- `p` subtitle (uses default text when `subtitle` is `None`)

Exported from `components/mod.rs` as `pub use coming_soon::ComingSoon;`.

## Styling

`assets/styling/coming_soon.css`

- `.coming-soon` wrapper: flex column, `align-items: center`, top padding for breathing
  room on the empty page.
- `.coming-soon__card`: pale yellow background `#faf3c5`, soft amber border `#e6d98a`
  (`0.0625rem solid`), `border-radius: 0.5rem`, generous padding, `text-align: center`,
  `max-width: ~22rem`.
- `.coming-soon__icon`: ~3rem, `object-fit: contain`.
- `.coming-soon__title`: ~1.25rem, `font-weight: 700`, dark `#2a2614`.
- `.coming-soon__subtitle`: ~0.875rem, muted dark `#5c5530`, `margin: 0`.

## Page Integration

Surgical edits — add the component as the child of the existing `PageLayout`.

```rust
// src/views/map.rs
PageLayout { title: "Map",
    ComingSoon { icon: asset!("/assets/styling/media/icons/map.svg") }
}

// src/views/raider.rs
PageLayout { title: "Raider",
    ComingSoon { icon: asset!("/assets/styling/media/icons/raider.svg") }
}
```

## Files Touched

- **New:** `src/components/coming_soon.rs`
- **New:** `assets/styling/coming_soon.css`
- **Edit:** `src/components/mod.rs` — register and export the module
- **Edit:** `src/views/map.rs` — render `ComingSoon` with `map.svg`
- **Edit:** `src/views/raider.rs` — render `ComingSoon` with `raider.svg`

## Out of Scope (YAGNI)

- No global theme tokens for yellow.
- No customizable heading text (always "Coming Soon").
- No animation/interactivity (it is a static notice).
- No changes to other pages.

## Verification

- `cargo check` (or the project build) passes.
- App builds and both Map and Raider pages render the centered pale-yellow card with the
  correct per-page icon, heading, and subtitle, in both dark and light themes.
