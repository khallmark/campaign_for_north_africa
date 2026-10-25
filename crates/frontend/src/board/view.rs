use super::{calendar, Board, Counter};
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
        r#"<button class="calendar-chip" data-action="calendar" aria-label="Open campaign calendar"><span>{}</span><small>Week / turn {} of 111 <i>·</i> Stage {} of 3 <i>·</i> Reference</small></button><nav class="board-tools" aria-label="Campaign tools">{}{}{}{}{}</nav>"#,
        calendar::label(b.turn),
        b.turn,
        b.stage,
        action("help", "Field manual", "book", false),
        action("catalog", "Unit catalog", "box", false),
        action("reinforcements", "Reinforcements", "plus", false),
        action("reset", "New operation", "reset", false),
        action(
            "end",
            "End stage — unavailable in review mode",
            "arrow",
            true
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
        format!(r#"<button class="board-roster-unit" data-counter="{}" aria-pressed="{}"><span class="roster-art">{}</span><span><strong>{}</strong><small>{}</small></span></button>"#, c.id, Some(c.id) == b.selected, portrait(b, c), escape(&identity.name), c.hex.map(|h| format!("{h} · Planning counter")).unwrap_or("Awaiting placement".into()))
    }).collect();
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
        let actions = if selected && !b.placing {
            format!(r#"<div class="counter-actions" role="group" aria-label="Selected unit actions">{}{}{}{}{}</div>"#,
                action("move-counter", "Reposition planning counter", "move", false),
                action("attack", "Attack — requires an active simulation", "attack", true),
                action("digin", "Dig in — requires an active simulation", "shield", true),
                action("resupply", "Resupply — requires an active simulation", "box", true),
                action("remove-counter", "Remove planning counter", "close", false))
        } else { String::new() };
        Some(format!(r#"<div class="board-counter {}" style="left:{}px;top:{}px" data-board-x="{}" data-board-y="{}"><button class="counter-marker" data-counter="{}" aria-label="Select {} planning counter" aria-pressed="{selected}">{}<span>{}</span></button>{actions}</div>"#, if selected { "selected" } else { "" }, p.x, p.y, point.x, point.y, c.id, escape(&identity.name), portrait(b,c), c.id))
    }).collect()
}

pub(super) fn dialog(b: &Board, name: &str) -> String {
    let (title, content) = match name {
        "catalog" => ("Unit catalog", b.catalog.panel()),
        "reinforcements" => ("Reinforcements", b.inventory.planning_panel()),
        "calendar" => ("Campaign calendar", calendar::panel(b.turn, b.stage)),
        "reset" => ("New operation", r#"<h2>Start a new review board?</h2><p>This clears your user-placed counters and resets the calendar reference to Game-Turn 1, Stage 1. No historical scenario will be loaded. This session is not saved.</p><div class="dialog-actions"><button data-action="close">Keep this board</button><button data-action="confirm-reset" class="primary">Clear review board</button></div>"#.into()),
        "unavailable" => ("Simulation not connected", r#"<h2>No active simulation</h2><p>This is a map and unit review workspace. Combat, supply, fortification and stage resolution are not connected to an official engine.</p><p>Planning counters never inherit the old teaching scenario’s statistics or positions. The calendar is a source reference, not an operational clock.</p><button data-action="close" class="primary">Return to map</button>"#.into()),
        _ => ("Field manual", r#"<h2>Field manual</h2><p>Explore the full supplied board, inspect source records and arrange planning counters. No fictional training scenario is running.</p><div class="manual-grid"><section><h3>Navigate the map</h3><p>Drag to pan. Scroll or pinch to zoom. With the map focused, arrow keys pan, + / − zoom and Home fits the entire board.</p></section><section><h3>Arrange your board</h3><p>Add an identity in Reinforcements. Click a mapped position, or pan and press Enter to place at the center. Select a counter to inspect it. Its icon actions follow it through pan and zoom; hover for one second or focus for a tooltip.</p></section><section><h3>Read the calendar</h3><p>Click the date to browse 111 weekly Game-Turns and three Operations Stages per turn. The reference cursor does not execute game phases. Original track annotations remain on the map.</p></section><section><h3>Know the limits</h3><p>These are candidate identities and user-chosen positions. Section-local IDs come from VASSAL geometry. Terrain, printed aliases, seams, historical deployments and official rules execution are not certified. Reloading clears this session.</p></section></div>"#.into()),
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
