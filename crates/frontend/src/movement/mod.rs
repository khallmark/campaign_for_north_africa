//! Formation order controls, owned by the movement workstream.
use crate::{view::button, App};
use cna_sim::*;

pub(crate) fn orders(app: &App, u: &Unit, source: bool) -> String {
    let friendly = u.side == "allied";
    let active = friendly && app.game.outcome.is_none();
    let enemies = app
        .game
        .units
        .iter()
        .any(|target| can_attack(&app.game, Some(u), Some(target)));
    if friendly {
        format!(
            r#"<div class="order-grid">{}{}{}{}{}</div><p class="order-hint">{}</p>"#,
            button(
                "move",
                "Move",
                "move",
                !active || reachable(&app.game, Some(u)).is_empty(),
                if app.mode == "move" { "active" } else { "" }
            ),
            button(
                "attack",
                if u.role().range > 1 {
                    "Attack"
                } else {
                    "Assault"
                },
                "attack",
                !active || !enemies,
                if app.mode == "attack" {
                    "active danger"
                } else {
                    ""
                }
            ),
            button(
                "digin",
                if u.entrenched { "Dug in" } else { "Dig in" },
                "shield",
                !active || u.cp < 1 || u.entrenched || !u.role().can_capture(),
                ""
            ),
            button(
                "resupply",
                "Resupply",
                "box",
                !active
                    || u.cp < 1
                    || !source
                    || (u.fuel == 100 && u.water == 100 && u.ammo == 100),
                ""
            ),
            if matches!(
                u.role().support,
                Support::Command | Support::Repair | Support::Fortify
            ) {
                button(
                    "support",
                    match u.role().support {
                        Support::Command => "Rally",
                        Support::Repair => "Repair",
                        _ => "Fortify",
                    },
                    "shield",
                    !active
                        || !app
                            .game
                            .units
                            .iter()
                            .any(|target| can_support(&app.game, Some(u), Some(target))),
                    if app.mode == "support" { "active" } else { "" },
                )
            } else {
                String::new()
            },
            match app.mode {
                "move" => "Click a highlighted hex to move. Numbers show CP cost.",
                "attack" => "Choose a marked enemy in range to preview an attack.",
                "support" => "Choose a marked adjacent friendly unit.",
                _ => u.role().description,
            }
        )
    } else {
        "<p class=order-hint>Enemy formations act when you end the stage. Select a Commonwealth formation to issue orders.</p>".into()
    }
}
