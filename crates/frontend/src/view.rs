use crate::App;
use cna_sim::*;

mod panel;

pub fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn icon(name: &str) -> String {
    let path = match name {
        "arrow" => r#"<path d="M4 12h16m-6-6 6 6-6 6"/>"#,
        "move" => {
            r#"<path d="M12 3v18M3 12h18M9 6l3-3 3 3M9 18l3 3 3-3M6 9l-3 3 3 3M18 9l3 3-3 3"/>"#
        }
        "attack" => {
            r#"<circle cx="12" cy="12" r="7"/><path d="M12 2v5m0 10v5M2 12h5m10 0h5"/><circle cx="12" cy="12" r="2"/>"#
        }
        "shield" => r#"<path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6Z"/><path d="m8 12 3 3 5-6"/>"#,
        "box" => r#"<path d="m12 3 9 5v9l-9 5-9-5V8Zm-9 5 9 5 9-5M12 13v9M7 5l9 5"/>"#,
        "sun" => {
            r#"<circle cx="12" cy="12" r="4"/><path d="M12 1v3m0 16v3M1 12h3m16 0h3M4 4l2 2m12 12 2 2M4 20l2-2M18 6l2-2"/>"#
        }
        "book" => r#"<path d="M12 5C8 2 3 3 3 3v16s5-1 9 2c4-3 9-2 9-2V3s-5-1-9 2Zm0 0v16"/>"#,
        "reset" => r#"<path d="M3 10a9 9 0 1 1 1 8M3 3v7h7"/>"#,
        "grid" => r#"<path d="m12 2 9 5v10l-9 5-9-5V7Z"/><path d="m3 7 9 5 9-5M12 12v10"/>"#,
        "plus" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "minus" => r#"<path d="M5 12h14"/>"#,
        "water" => {
            r#"<path d="M12 2S5 10 5 15a7 7 0 0 0 14 0c0-5-7-13-7-13Z"/><path d="M8 15a4 4 0 0 0 4 4"/>"#
        }
        "fuel" => r#"<path d="M5 3h10v18H5Zm0 7h10M3 21h14M15 7h3l3 3v8c0 2-3 2-3 0v-5h-3"/>"#,
        "ammo" => r#"<path d="M5 21V7l3-5 3 5v14ZM15 21V7l3-5 3 5v14ZM5 15h6m4 0h6"/>"#,
        "close" => r#"<path d="m6 6 12 12M6 18 18 6"/>"#,
        "flag" => r#"<path d="M5 22V3m0 0c5-4 9 4 15 0v10c-6 4-10-4-15 0"/>"#,
        _ => r#"<path d="m12 2 3 7 7 3-7 3-3 7-3-7-7-3 7-3Z"/><circle cx="12" cy="12" r="2"/>"#,
    };
    format!(
        r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{path}</svg>"#
    )
}
pub(crate) fn button(
    action: &str,
    label: &str,
    symbol: &str,
    disabled: bool,
    class: &str,
) -> String {
    format!(
        r#"<button data-action="{action}" aria-label="{label}" {} class="{class}">{}<span>{label}</span></button>"#,
        if disabled { "disabled" } else { "" },
        icon(symbol)
    )
}
fn modal(app: &App) -> String {
    let Some(modal) = app.modal else {
        return String::new();
    };
    let body = match modal {
        "catalog" => crate::catalog_view::catalog(app),
        "reinforcements" => crate::catalog_view::reinforcements(app),
        "help" => format!(
            r#"<span class="eyebrow">Field manual / prototype 0.1</span><h2>A war of distance.<br>A battle for supplies.</h2><p>Lead the Commonwealth west to Tobruk. This is a small, fictional training scenario inspired by the land and logistics rules of <em>The Campaign for North Africa</em>, not a historical order of battle.</p>
        <div class="manual-grid"><section>{}<h3>Move with purpose</h3><p>Select a green formation, then Move. Desert costs 1 capability point (CP); ridges cost 2. Vehicles spend 8 fuel per CP. Entering an enemy’s adjacent hex ends that move.</p></section><section>{}<h3>Fight at close range</h3><p>Assault an adjacent enemy for 2 CP and 20 ammunition. Preview exact damage before committing. Dig in for 1 CP to reduce incoming damage by 10; moving or assaulting removes cover.</p></section><section>{}<h3>Mind the supply line</h3><p>Resupply for 1 CP within 2 hexes of a stocked truck or the depot. Trucks transfer up to 25 of each resource; the depot refills stores. Zero water prevents movement and assaults.</p></section><section>{}<h3>The desert takes a toll</h3><p>End stage activates the Axis, consumes 10 water and 2 vehicle fuel, then restores CP. Every third stage costs 18 water and 5 fuel instead. Zero water causes 10 strength loss.</p></section></div>
        <div class="manual-objective">{}<p><strong>Secure Tobruk within {STAGE_LIMIT} operations stages.</strong><br>Move a combat formation into its hex after defeating the garrison. Lose if time runs out or all combat formations are destroyed.</p></div>
        <p>Reinforcements lets you place named candidate identities or fictional role units. Sandbox aircraft overfly land, sea and ground control; ships stay at sea. Attack range and permitted targets depend on role. Engineers fortify adjacent ground combat units; HQ restores one CP for two CP; recovery vehicles restore twenty strength for one CP and twenty fuel. Ground support units transfer supplies like trucks.</p>
        <p class="fine-print">Inspired by rules §§5, 6, 8, 15, 49 and 52. Three stages represent a week. Map, forces, costs, weather, and combat are simplified. Air/naval actions are demonstrations, not the full rulebook systems. Detailed logistics and the pasta rule are not implemented. Unique unit art is incomplete and placeholders are marked. Session resets on page reload.</p>{}"#,
            icon("move"),
            icon("attack"),
            icon("box"),
            icon("sun"),
            icon("flag"),
            button("close", "Return to the map", "arrow", false, "primary")
        ),
        "reset" => format!(
            r#"<span class="eyebrow">A fresh set of orders</span><h2>Start a new operation?</h2><p>This will reset all positions, supplies, and the field journal. The current session will be lost.</p><div class="dialog-actions">{}{}</div>"#,
            button("close", "Keep playing", "close", false, ""),
            button(
                "confirm-reset",
                "Start new operation",
                "reset",
                false,
                "primary"
            )
        ),
        "combat" => {
            let a = app.selected().unwrap();
            let b = app.game.units.iter().find(|u| u.id == app.target).unwrap();
            let preview = combat_preview(a, b);
            format!(
                r#"<span class="eyebrow">Attack / {} → {}</span><h2>Commit to the attack?</h2><p>{} will engage {}.</p><div class="combat-comparison"><div><img src="{}" alt="Attacking formation"/><span>Player forces</span><strong>{}% strength</strong><small>{} return damage</small></div><span class="versus">vs.</span><div><img src="{}" alt="Defending formation"/><span>Enemy forces</span><strong>{}% strength</strong><small>−{}% on attack</small></div></div><p class="combat-cost">Cost: <strong>2 CP</strong> and <strong>20 ammunition</strong>. {}</p><div class="dialog-actions">{}{}</div>"#,
                escape(&a.short),
                escape(&b.short),
                escape(&a.name),
                escape(&b.name),
                escape(&app.inventory.portrait(a)),
                a.hp,
                if b.hp <= preview.damage {
                    "No".into()
                } else {
                    format!("{}%", preview.retaliation)
                },
                escape(&app.inventory.portrait(b)),
                b.hp,
                preview.damage,
                if b.hp <= preview.damage {
                    "This attack will destroy the enemy formation."
                } else if preview.retaliation == 0 {
                    "The defender cannot return fire at this range or against this target."
                } else {
                    "The defender will return fire if it survives."
                },
                button("close", "Cancel", "close", false, ""),
                button(
                    "confirm-attack",
                    "Launch attack",
                    "attack",
                    false,
                    "primary"
                )
            )
        }
        "outcome" => {
            let victory = app.game.outcome.as_deref() == Some("victory");
            format!(
                r#"<span class="eyebrow">Operation report</span><div class="outcome-symbol">{}</div><h2>{}</h2><p>{}</p><p>Stages played: {} · Surviving formations: {}/{}</p><div class="dialog-actions">{}{}</div>"#,
                icon("flag"),
                if victory {
                    "Tobruk is secured."
                } else {
                    "The advance has ended."
                },
                if victory {
                    "Your formations have taken the objective. The road to Tobruk is open."
                } else if app.game.stage > STAGE_LIMIT {
                    "The operational window has closed. Tobruk remains beyond your lines."
                } else {
                    "Your last combat formation has been lost. The operation cannot continue."
                },
                app.game.stage.min(STAGE_LIMIT),
                app.game
                    .units
                    .iter()
                    .filter(|u| u.side == "allied" && u.hp > 0)
                    .count(),
                app.game.units.iter().filter(|u| u.side == "allied").count(),
                button("close", "Review the map", "grid", false, ""),
                button(
                    "confirm-reset",
                    "Try a new operation",
                    "reset",
                    false,
                    "primary"
                )
            )
        }
        _ => String::new(),
    };
    format!(
        r#"<dialog class="dialog {}" aria-label="{}"><button class="dialog-close icon-button" data-action="close" aria-label="Close dialog">{}</button>{body}</dialog>"#,
        match modal {
            "help" => "manual",
            "catalog" => "catalog",
            "reinforcements" => "catalog",
            _ => "",
        },
        match modal {
            "help" => "Field manual",
            "catalog" => "Unit catalog",
            "reinforcements" => "Reinforcements",
            "combat" => "Assault preview",
            "outcome" => "Operation result",
            _ => "New operation",
        },
        icon("close")
    )
}

pub fn page(app: &App) -> String {
    let hot = app.game.stage % 3 == 0;
    let friendlies: Vec<_> = app
        .game
        .units
        .iter()
        .filter(|u| u.side == "allied")
        .collect();
    let roster: String = friendlies.iter().map(|u| format!(r#"<button class="roster-unit {} {}" data-unit="{}" aria-label="Select {}" aria-pressed="{}" {}><img src="{}" alt=""/><div><strong>{}</strong><span>{}</span></div><span class="roster-dot {}"></span></button>"#,
        if u.id==app.selected{"chosen"}else{""},if u.hp<=0{"destroyed"}else{""},u.id,escape(&u.name),u.id==app.selected,if u.hp<=0{"disabled"}else{""},escape(&app.inventory.portrait(u)),if u.short=="RASC"{"Supply column".into()}else{escape(&u.name.replace(" Brigade",""))},if u.hp<=0{"Formation lost".into()}else{format!("{}% strength <i>·</i> {} CP",u.hp,u.cp)},if u.cp==0{"spent"}else{""})).collect();
    let journal: String = app.game.log.iter().take(3).map(|e|format!(r#"<div class="journal-entry {}"><span class="journal-tick"></span><p>{}</p><time>S{}</time></div>"#,escape(&e.log_type),escape(&e.text),e.stage)).collect();
    let order_mode = if app.mode != "inspect" {
        format!(
            r#"<div class="map-mode">{}<span>{}</span><button data-action="cancel" aria-label="Cancel order">{}</button></div>"#,
            icon(if app.mode == "move" { "move" } else { "attack" }),
            match app.mode {
                "move" => "Choose a highlighted destination",
                "deploy" => "Place the reinforcement on an empty highlighted hex",
                "support" => "Choose a marked friendly unit",
                _ => "Choose a marked enemy in range",
            },
            icon("close")
        )
    } else {
        String::new()
    };
    format!(
        r#"<header class="site-header"><a href="/" class="brand" aria-label="The Campaign for North Africa home"><span class="brand-mark">{compass}</span><div><span class="brand-prefix">The Campaign for</span><span class="brand-name">North Africa</span></div></a><div class="edition"><span class="edition-line"></span><span>A desert warfare simulation<small>1940 — 1943</small></span></div><nav aria-label="Game controls">{help}{catalog}{reinforcements}{reset}</nav></header>
    <main><section class="operation-bar"><div class="operation-heading"><div class="scenario-meta"><span class="live-dot"></span>Sandbox scenario<span class="separator">/</span><span>Western Desert</span></div><h1>The road to Tobruk</h1></div><div class="turn-controls"><div class="weather">{sun}<div>{weather}<small>{loss}</small></div></div><div class="turn-date"><strong>{date}</strong><span>Week {week} <i>·</i> Stage {stage} of 3</span></div>{end}</div></section>
    <div class="game-layout"><section class="map-panel" aria-label="Battlefield"><div class="map-toolbar"><div class="map-title">{compass}<span>Operational map</span><span class="map-tag">Tobruk sector</span></div><div class="map-toggles"><button data-action="grid" aria-label="Toggle hex grid" aria-pressed="{grid}" class="icon-button {grid_class}" title="Hex grid">{grid_icon}</button><button data-action="supplies" aria-label="Toggle supply range" aria-pressed="{supplies}" class="icon-button {supply_class}" title="Supply range">{box_icon}</button></div></div>
    <div class="map-viewport"><div id="map-slot"></div><div class="objective-note">{flag}<div><strong>Secure Tobruk</strong><span>{remaining} stages remaining</span></div></div><div class="map-zoom"><button data-action="zoom-in" aria-label="Zoom in" {max_zoom}>{plus}</button><button data-action="zoom-reset" aria-label="Reset map zoom">{zoom}%</button><button data-action="zoom-out" aria-label="Zoom out" {min_zoom}>{minus}</button></div>{order_mode}</div>
    <div class="map-legend"><span><i class="faction-dot allied"></i>Commonwealth</span><span><i class="faction-dot axis"></i>Axis</span><span class="legend-ridge">△<span>Ridge · 2 CP</span></span><span class="map-scale-caption">Schematic map · ~8 km / hex</span></div><div class="orders-status {error}" role="status" aria-live="polite">{status_icon}<span>{message}</span></div></section><aside class="detail-panel" aria-label="Selected formation">{panel}</aside></div>
    <section class="bottom-layout"><div class="roster-section"><div class="section-title"><h2>Order of battle <span>{count} formations</span></h2><span class="your-command"><i class="faction-dot allied"></i>Your command</span></div><div class="roster">{roster}</div></div><section class="journal"><div class="section-title"><h2>Field journal</h2><span>Latest dispatches</span></div><div class="journal-entries">{journal}</div></section></section></main><footer><span>Distance is the enemy. Supply is the lifeline.</span><span>Playable prototype <i>·</i> Simplified rules <i>·</i> Session only</span></footer>{modal}"#,
        compass = icon("compass"),
        help = button("help", "Field manual", "book", false, ""),
        catalog = button("catalog", "Unit catalog", "box", false, ""),
        reinforcements = button(
            "reinforcements",
            "Reinforcements",
            "plus",
            app.game.outcome.is_some(),
            ""
        ),
        reset = button("reset", "New operation", "reset", false, ""),
        sun = icon("sun"),
        weather = if hot { "Hot winds" } else { "Clear skies" },
        loss = if hot {
            "Increased supply loss"
        } else {
            "Normal supply loss"
        },
        date = date_label(app.game.stage),
        week = (app.game.stage + 2) / 3,
        stage = (app.game.stage - 1) % 3 + 1,
        end = button(
            if app.game.outcome.is_some() {
                "outcome"
            } else {
                "end"
            },
            if app.game.outcome.is_some() {
                "View result"
            } else {
                "End stage"
            },
            "arrow",
            false,
            "primary end-stage"
        ),
        grid = app.grid,
        grid_class = if app.grid { "toggled" } else { "" },
        grid_icon = icon("grid"),
        supplies = app.supplies,
        supply_class = if app.supplies { "toggled" } else { "" },
        box_icon = icon("box"),
        flag = icon("flag"),
        remaining = (STAGE_LIMIT - app.game.stage + 1).max(0),
        max_zoom = if app.zoom >= 1.75 { "disabled" } else { "" },
        min_zoom = if app.zoom <= 1.0 { "disabled" } else { "" },
        zoom = (app.zoom * 100.0).round(),
        plus = icon("plus"),
        minus = icon("minus"),
        error = if app.error { "error" } else { "" },
        status_icon = icon(if app.error { "flag" } else { "compass" }),
        message = escape(&app.message),
        panel = panel::panel(app),
        count = friendlies.iter().filter(|u| u.hp > 0).count(),
        modal = modal(app)
    )
}
