use dioxus::prelude::*;

const PAGE_LAYOUT_CSS: Asset = asset!("/assets/styling/page_layout.css");

/// A generic page layout component that provides consistent structure across all pages.
///
/// # Props
/// - `title`: The page title rendered as an `h1` at the top.
/// - `children`: The page content rendered below the title with consistent spacing.
#[component]
pub fn PageLayout(title: String, children: Element) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: PAGE_LAYOUT_CSS }

        div {
            class: "page-layout",
            style: "display: flex; flex-direction: column; width: 100%; padding: 0 3vw; gap: 4vw;",
            h1 {
                class: "page-layout__title",
                style: "font-size: 1.5rem; font-weight: 700; color: var(--color-text-primary);",
                "{title}"
            }
            div {
                class: "page-layout__content",
                style: "display: flex; flex-direction: column; width: 100%; gap: 0.25rem;",
                {children}
            }
        }
    }
}
