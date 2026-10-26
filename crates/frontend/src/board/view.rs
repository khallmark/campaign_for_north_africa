use super::{calendar, play, Board, Counter};
use crate::view::{escape, icon};
use cna_map::{geometry, viewport::Viewport};

fn action(name: &str, label: &str, symbol: &str, unavailable: bool) -> String {
    format!(
        r#"<button data-action="{name}" class="board-icon {}" aria-label="{label}" aria-describedby="tip-{name}">{}<span class="board-tooltip" role="tooltip" id="tip-{name}">{label}</span></button>"#,
        if unavailable {
            "unavailable-action"
        } else {
            ""
        },
        icon(symbol)
    )
}

pub(super) fn top(b: &Board) -> String {
    format!(
        r#"<button class="calendar-chip" data-action="calendar" aria-label="Open campaign calendar"><span>{}</span><small>Week / turn {} of 111 <i>·</i> Stage {} of 3 <i>·</i> {}</small></button><nav class="board-tools" aria-label="Campaign tools"><button data-action="play">Play / Save</button>{}{}{}{}{}</nav>"#,
        calendar::label(b.turn),
        b.turn,
        b.stage,
        b.game
            .as_ref()
            .map(play::status)
            .unwrap_or("Reference".into()),
        action("help", "Field manual", "book", false),
        action("catalog", "Unit catalog", "box", false),
        action("reinforcements", "Reinforcements", "plus", false),
        action("reset", "New operation", "reset", false),
        action(
            "end",
            if b.game.is_some() {
                "Complete player portion / advance stage"
            } else {
                "End stage — unavailable in review mode"
            },
            "arrow",
            b.game.is_none()
        )
    )
}

fn portrait(b: &Board, c: &Counter) -> String {
    let entry = b.inventory.find(&c.identity).unwrap();
    let path = if entry.sprite.ready() {
        entry.sprite.portrait_path.clone()
    } else {
        entry
            .role_id
            .as_deref()
            .and_then(cna_sim::role)
            .map(|r| format!("/sprites/{}.png", r.id))
    };
    path.map(|path| format!(r#"<img src="{}" alt=""/>"#, escape(&path)))
        .unwrap_or_else(|| icon("flag"))
}

pub(super) fn roster(b: &Board) -> String {
    let rows: String = b.counters.iter().map(|c| {
        let identity = b.inventory.find(&c.identity).unwrap();
        let position = b.play_piece(c.id).map(|p| format!("{} · {}{}", p.hex, cna_sim::tabletop::side_name(p.side), if p.eliminated { " · Eliminated" } else { "" }))
            .unwrap_or_else(|| c.hex.map(|h| format!("{h} · Planning counter")).unwrap_or("Awaiting placement".into()));
        format!(r#"<button class="board-roster-unit" data-counter="{}" aria-pressed="{}"><span class="roster-art">{}</span><span><strong>{}</strong><small>{}</small></span></button>"#, c.id, Some(c.id) == b.selected, portrait(b, c), escape(&identity.name), position)
    }).collect();
    if let Some(game) = &b.game {
        return format!("<p class=\"panel-caption\">{}</p>{rows}<p class=\"panel-footnote\">Player-adjudicated operation. Use Play / Save for dice, order log and backup.</p>", play::status(game));
    }
    format!(
        r#"<p class="panel-caption">{} planning counters</p>{}<button class="add-counter" data-action="reinforcements">{} Add a counter</button><p class="panel-footnote">No historical deployment loaded.</p>"#,
        b.counters.len(),
        if rows.is_empty() {
            "<div class=\"panel-empty\"><p>Your board is clear.</p><p>Add an identity from Reinforcements to review its artwork and plan a position.</p></div>"
        } else {
            &rows
        },
        icon("plus")
    )
}

pub(super) fn detail(b: &Board) -> String {
    let Some(c) = b.selected() else {
        return format!(
            r#"<div class="panel-empty empty-unit">{}<h2>Select a unit</h2><p>Choose a counter on the map or in the order of battle.</p><p>Historical positions and an official simulation are not loaded.</p></div>"#,
            icon("flag")
        );
    };
    let identity = b.inventory.find(&c.identity).unwrap();
    if let Some(piece) = b.play_piece(c.id) {
        let controls = if b.can_order(c.id) {
            r#"<div class="play-unit-actions"><button data-action="move-counter">Move</button><button data-action="attack">Combat result</button><button data-action="resupply">Supply ledger</button><button data-action="digin">Construction</button></div>"#
        } else {
            "<p>Only the active player’s surviving counters can receive orders.</p>"
        };
        return format!("<div class=\"board-unit-art\">{}</div><h2>{}</h2><p>{}</p>{}{controls}<p class=\"panel-footnote\">Player-adjudicated state; partial losses and rule-specific effects remain in the order log / rulebook records.</p>", portrait(b, c), escape(&identity.name), piece.hex, play::detail(piece));
    }
    format!(
        r#"<div class="board-unit-art">{}</div><h2>{}</h2><p class="unit-nationality">{} · User-placed</p><dl class="unit-facts"><div><dt>Position</dt><dd>{}</dd></div><div><dt>Identity</dt><dd>{}</dd></div><div><dt>Artwork</dt><dd>{}</dd></div></dl><p class="panel-footnote">VASSAL-derived section ID, not a certified printed alias. No historical strength, supplies or movement allowance assigned.</p><div class="unit-local-controls">{}</div>"#,
        portrait(b, c),
        escape(&identity.name),
        escape(&identity.side),
        c.hex.map(|h| h.to_string()).unwrap_or("Not placed".into()),
        escape(&identity.identity_status),
        if identity.sprite.ready() {
            "Generated identity illustration"
        } else {
            "Generated illustration missing; reference placeholder"
        },
        if c.hex.is_some() {
            action("locate", "Locate selected unit", "attack", false)
        } else {
            "<button data-action=\"move-counter\">Place counter</button>".into()
        }
    )
}

pub(super) fn counters(b: &Board, viewport: Viewport) -> String {
    let [width, height] = viewport.css_size();
    b.counters.iter().filter_map(|c| {
        let point = geometry::center(c.hex?);
        let p = viewport.to_screen(point);
        // Hide offscreen counters rather than pinning their actions to an unrelated map position.
        if p.x < -40.0 || p.x > width + 40.0 || p.y < -40.0 || p.y > height + 40.0 { return None; }
        let selected = Some(c.id) == b.selected;
        let identity = b.inventory.find(&c.identity).unwrap();
        let actions = if selected && !b.placing && b.game.is_some() && b.can_order(c.id) {
            format!(r#"<div class="counter-actions" role="group" aria-label="Selected unit actions">{}{}{}{}</div>"#,
                action("move-counter", "Record move", "move", false),
                action("attack", "Record combat result", "attack", false),
                action("digin", "Record construction", "shield", false),
                action("resupply", "Record supplies", "box", false))
        } else if selected && !b.placing && b.game.is_none() {
            format!(r#"<div class="counter-actions" role="group" aria-label="Selected unit actions">{}{}{}{}{}</div>"#,
                action("move-counter", "Reposition planning counter", "move", false),
                action("attack", "Attack — requires an active simulation", "attack", true),
                action("digin", "Dig in — requires an active simulation", "shield", true),
                action("resupply", "Resupply — requires an active simulation", "box", true),
                action("remove-counter", "Remove planning counter", "close", false))
        } else { String::new() };
        Some(format!(r#"<div class="board-counter {}" style="left:{}px;top:{}px" data-board-x="{}" data-board-y="{}"><button class="counter-marker" data-counter="{}" aria-label="Select {} {} counter" aria-pressed="{selected}">{}<span>{}</span></button>{actions}</div>"#, if selected { "selected" } else { "" }, p.x, p.y, point.x, point.y, c.id, escape(&identity.name), if b.game.is_some() { "game" } else { "planning" }, portrait(b,c), c.id))
    }).collect()
}

pub(super) fn dialog(b: &Board, name: &str) -> String {
    let (title, content) = match name {
        "play" | "play-move" | "play-combat" | "play-supplies" | "play-digin" | "play-end" | "play-concede" => play::dialog(b, name),
        "catalog" => ("Unit catalog", b.catalog.panel()),
        "reinforcements" => ("Reinforcements", b.inventory.planning_panel()),
        "calendar" => ("Campaign calendar", calendar::panel(b.turn, b.stage)),
        "reset" => ("New operation", r#"<h2>Start a new review board?</h2><p>This clears the current board and any saved hot-seat game in this browser. Export it from Play / Save first if you want to keep it. The calendar resets to Game-Turn 1, Stage 1. No historical scenario will be loaded.</p><div class="dialog-actions"><button data-action="close">Keep this board</button><button data-action="confirm-reset" class="primary">Clear review board</button></div>"#.into()),
        "unavailable" => ("Simulation not connected", r#"<h2>No active simulation</h2><p>This is a map and unit review workspace. Combat, supply, fortification and stage resolution are not connected to an official engine.</p><p>Planning counters never inherit the old teaching scenario’s statistics or positions. The calendar is a source reference, not an operational clock.</p><button data-action="close" class="primary">Return to map</button>"#.into()),
        _ => ("Field manual", r#"<h2>Field manual</h2><p>Explore the full supplied board, inspect source records and arrange planning counters. No fictional training scenario is running.</p><div class="manual-grid"><section><h3>Navigate the map</h3><p>Drag to pan. Scroll or pinch to zoom. With the map focused, arrow keys pan, + / − zoom and Home fits the entire board.</p></section><section><h3>Arrange your board</h3><p>Add an identity in Reinforcements. Click a mapped position, or pan and press Enter to place at the center. Select a counter to inspect it. Its icon actions follow it through pan and zoom; hover for one second or focus for a tooltip.</p></section><section><h3>Read the calendar</h3><p>Click the date to browse 111 weekly Game-Turns and three Operations Stages per turn before starting a game. The reference cursor does not execute game phases. Original track annotations remain on the map.</p></section><section><h3>Play together</h3><p>Place both sides, then choose Play / Save to start a rulebook hot-seat game. Record adjudicated orders, pass the browser to the other player, and confirm both portions before advancing the stage. The game autosaves and resumes on reload; Play / Save provides the log and JSON backup. Concession ends play.</p></section><section><h3>Know the limits</h3><p>These are candidate identities and user-chosen positions. Section-local IDs come from VASSAL geometry. Terrain, printed aliases, seams, historical deployments and official rules execution are not certified. Rulebook play requires players to adjudicate rules; review-only counters reset on reload.</p></section></div>"#.into()),
    };
    format!(
        r#"<dialog class="dialog board-dialog {}" aria-label="{title}"><button class="dialog-close icon-button" data-action="close" aria-label="Close dialog">{}</button>{content}</dialog>"#,
        if name == "catalog" || name == "reinforcements" {
            "catalog"
        } else {
            ""
        },
        icon("close")
    )
}
