use crate::{inspect::side, movement::orders, App};
use cna_sim::*;

use super::{escape, icon};

pub(crate) fn panel(app: &App) -> String {
    let Some(u) = app.selected() else {
        return "<div class=empty-panel><h2>No formation selected</h2><p>Select a surviving formation on the map or in the order of battle.</p></div>".into();
    };
    let friendly = u.side == "allied";
    let source = supply_source(&app.game, Some(u)).is_some();
    let cp: String = (0..u.max_cp)
        .map(|i| {
            format!(
                r#"<i class="{}"></i>"#,
                if i < u.cp { "available" } else { "" }
            )
        })
        .collect();
    let resources: String = [("fuel", "Fuel", u.fuel), ("water", "Water", u.water), ("ammo", "Ammunition", u.ammo)].iter().map(|(key, label, value)| {
        format!(r#"<div class="resource-row {}">{}<span>{label}</span><div class="resource-track"><i style="width:{value}%"></i></div><strong>{}</strong></div>"#, if *value < 25 { "low" } else { "" }, icon(key), if !u.role().motorized && *key == "fuel" { "—".into() } else { format!("{value}%") })
    }).collect();
    let orders = orders(app, u, source);
    format!(
        r#"<div class="panel-heading"><span>Formation details</span><span class="hex-coordinate">{:02}:{:02}</span></div>
    <section class="unit-detail"><div class="allegiance"><i class="faction-dot {}"></i>{}<span>{}</span></div>
    <div class="unit-portrait"><div class="portrait-ring"></div><img src="{}" alt="{}"/><span class="unit-stamp">{}</span></div>
    <h2>{}</h2><p class="unit-subtitle">{} · {} · range {}</p><p class="artwork-status">{}</p><div class="strength-line"><span>Sandbox strength</span><strong>{}<small>%</small></strong></div>
    <div class="strength-track"><span style="width:{}%"></span></div><div class="capability"><span>Capability points</span><div aria-label="{} of {} capability points">{cp}<strong>{}/{}</strong></div></div>{}</section>
    <section class="supplies-panel"><div class="section-title"><h3>Field supplies</h3><span class="supply-state {}">{}</span></div>{resources}{}</section>
    <section class="orders-panel"><h3>{}</h3>{orders}</section>"#,
        u.c + 1,
        u.r + 1,
        u.side,
        side(u),
        match u.unit_type.as_str() {
            "tank" => "▱",
            "infantry" => "×",
            _ => "◇",
        },
        escape(&app.inventory.portrait(u)),
        u.role().name,
        escape(&u.short),
        escape(&u.name),
        u.role().name,
        u.role().domain_name(),
        u.role().range,
        app.inventory.artwork_label(u),
        u.hp,
        u.hp,
        u.cp,
        u.max_cp,
        u.cp,
        u.max_cp,
        if u.entrenched {
            "<div class=dug-in>Dug in · 10 damage protection</div>"
        } else {
            ""
        },
        if source { "" } else { "isolated" },
        if !friendly {
            "Enemy stores"
        } else if source {
            "In supply range"
        } else {
            "Out of supply range"
        },
        if friendly {
            format!(
                "<p class=supply-note>{}</p>",
                if u.role().support == Support::Supply {
                    "Stocked trucks supply formations within 2 hexes. Refill at the frontier depot."
                } else {
                    "Keep your supply trucks close. The desert takes its share every stage."
                }
            )
        } else {
            String::new()
        },
        if friendly {
            "Issue orders"
        } else {
            "Reconnaissance"
        }
    )
}
