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
