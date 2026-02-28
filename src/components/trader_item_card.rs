use dioxus::prelude::*;

const TRADER_ITEM_CARD_CSS: Asset = asset!("/assets/styling/trader_item_card.css");

/// A compact card component for displaying a single trader item in a scrollable list.
///
/// Layout: [icon] [name / type · rarity] [price]
///
/// # Props
/// - `name`: The item's display name.
/// - `icon_url`: The URL to the item's icon image.
/// - `rarity`: The item's rarity tier (e.g. "Common", "Rare").
/// - `item_type`: The item's category (e.g. "Weapon", "Armor").
/// - `value`: The item's base value.
/// - `trader_price`: The price the trader sells it for.
#[component]
pub fn TraderItemCard(
    name: String,
    icon_url: String,
    rarity: String,
    item_type: String,
    value: i32,
    trader_price: i32,
) -> Element {
    let rarity_class = format!(
        "trader-item-card trader-item-card--{}",
        rarity.to_lowercase().replace(' ', "-")
    );

    rsx! {
        document::Link { rel: "stylesheet", href: TRADER_ITEM_CARD_CSS }

        div {
            class: "{rarity_class}",

            img {
                class: "trader-item-card__icon",
                src: "{icon_url}",
                alt: "{name}",
            }

            div {
                class: "trader-item-card__info",
                span {
                    class: "trader-item-card__name",
                    "{name}"
                }
                div {
                    class: "trader-item-card__meta",
                    span {
                        class: "trader-item-card__type",
                        "{item_type}"
                    }
                    span {
                        class: "trader-item-card__separator",
                        "·"
                    }
                    span {
                        class: "trader-item-card__rarity",
                        "{rarity}"
                    }
                }
            }

            div {
                class: "trader-item-card__price",
                span {
                    class: "trader-item-card__trader-price",
                    "{trader_price}"
                }
                span {
                    class: "trader-item-card__value",
                    "Val: {value}"
                }
            }
        }
    }
}
