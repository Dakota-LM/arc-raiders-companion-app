use arc_api_rs::models::items::StatBlock;
use dioxus::prelude::*;

/// Formats a stat value for display, removing unnecessary decimal places.
fn format_stat(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i32)
    } else {
        format!("{:.1}", value)
    }
}

/// Extracts non-zero stats from a `StatBlock` as `(name, formatted_value)` pairs.
///
/// Call this before rendering an `ItemCard` and pass the result as the `stats` prop.
pub fn extract_stats(stats: &StatBlock) -> Vec<(String, String)> {
    let candidates = [
        ("Damage", stats.damage),
        ("DPS", stats.damage_per_second),
        ("Fire Rate", stats.fire_rate),
        ("Range", stats.range),
        ("Magazine", stats.magazine_size),
        ("Stability", stats.stability),
        ("Health", stats.health),
        ("Shield", stats.shield),
        ("Shield Charge", stats.shield_charge),
        ("Healing", stats.healing),
        ("Healing/s", stats.healing_per_second),
        ("Stamina", stats.stamina),
        ("Stamina/s", stats.stamina_per_second),
        ("Duration", stats.duration),
        ("Weight", stats.weight),
        ("Weight Limit", stats.weight_limit),
        ("Agility", stats.agility),
        ("Stealth", stats.stealth),
        ("ARC Stun", stats.arc_stun),
        ("Raider Stun", stats.raider_stun),
        ("Use Time", stats.use_time),
        ("Damage Mult", stats.damage_mult),
        ("Augment Slots", stats.augment_slots),
        ("Backpack Slots", stats.backpack_slots),
        ("Quick Use Slots", stats.quick_use_slots),
        ("Safe Pocket Slots", stats.safe_pocket_slots),
        ("Healing Slots", stats.healing_slots),
        ("Stack Size", stats.stack_size),
        ("Damage Mitigation", stats.damage_mitigation),
        ("Movement Penalty", stats.movement_penalty),
        ("Reduced Noise", stats.reduced_noise),
        ("Reduced Reload", stats.reduced_reload_time),
        ("Reduced Recoil", stats.reduced_vertical_recoil),
        ("ADS Speed", stats.increased_ads_speed),
        ("Illumination", stats.illumination_radius),
    ];

    candidates
        .into_iter()
        .filter(|(_, v)| *v != 0.0)
        .map(|(name, value)| (name.to_string(), format_stat(value)))
        .collect()
}

#[component]
pub fn ItemCard(
    id: String,
    name: String,
    icon_url: String,
    rarity: String,
    item_type: String,
    value: i32,
    description: String,
    flavor_text: Option<String>,
    stats: Vec<(String, String)>,
    workbench: Option<String>,
    ammo_type: Option<String>,
    loot_area: Option<String>,
    #[props(default = false)]
    hide_value: bool,
    is_expanded: bool,
    on_toggle: EventHandler<String>,
) -> Element {
    let rarity_class = format!(
        "item-card item-card--{}",
        rarity.to_lowercase().replace(' ', "-")
    );

    let card_id = id.clone();

    rsx! {
        div {
            class: "{rarity_class}",
            onclick: move |_| on_toggle.call(card_id.clone()),

            // Collapsed summary row
            div {
                class: "item-card__summary",

                img {
                    class: "item-card__icon",
                    src: "{icon_url}",
                    alt: "{name}",
                }

                div {
                    class: "item-card__info",
                    span {
                        class: "item-card__name",
                        "{name}"
                    }
                    div {
                        class: "item-card__meta",
                        span {
                            class: "item-card__type",
                            "{item_type}"
                        }
                        span { "·" }
                        span {
                            class: "item-card__rarity",
                            "{rarity}"
                        }
                    }
                }

                if !hide_value {
                    div {
                        class: "item-card__value",
                        span {
                            class: "item-card__value-amount",
                            "{value}"
                        }
                    }
                }
            }

            // Expanded detail section — built only when this card is open, so
            // collapsed cards stay lightweight and hundreds render quickly.
            if is_expanded {
                div {
                    class: "item-card__details item-card__details--open",

                    // Description
                    if !description.is_empty() {
                        div {
                            class: "item-card__detail-section",
                            span { class: "item-card__detail-label", "Description" }
                            span { class: "item-card__detail-text", "{description}" }
                        }
                    }

                    // Flavor text
                    if let Some(ref flavor) = flavor_text {
                        div {
                            class: "item-card__detail-section",
                            span { class: "item-card__detail-label", "Flavor" }
                            span {
                                class: "item-card__detail-text",
                                style: "font-style: italic;",
                                "{flavor}"
                            }
                        }
                    }

                    // Stats grid
                    if !stats.is_empty() {
                        div {
                            class: "item-card__detail-section",
                            span { class: "item-card__detail-label", "Stats" }
                            div {
                                class: "item-card__stat-grid",
                                for (stat_name, stat_value) in stats.iter() {
                                    div {
                                        class: "item-card__stat",
                                        span { class: "item-card__stat-name", "{stat_name}" }
                                        span { class: "item-card__stat-value", "{stat_value}" }
                                    }
                                }
                            }
                        }
                    }

                    // Metadata row: workbench, ammo type, loot area
                    if workbench.is_some() || ammo_type.is_some() || loot_area.is_some() {
                        div {
                            class: "item-card__detail-section",
                            span { class: "item-card__detail-label", "Details" }
                            if let Some(ref wb) = workbench {
                                span { class: "item-card__detail-text", "Workbench: {wb}" }
                            }
                            if let Some(ref ammo) = ammo_type {
                                span { class: "item-card__detail-text", "Ammo: {ammo}" }
                            }
                            if let Some(ref area) = loot_area {
                                span { class: "item-card__detail-text", "Loot Area: {area}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
