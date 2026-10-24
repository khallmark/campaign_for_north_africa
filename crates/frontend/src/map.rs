use crate::{
    document,
    motion::{directional_frame, Motion, Pose},
    now,
    view::escape,
    App,
};
use cna_sim::*;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::Element;

mod research;
pub use research::mount as mount_research;

// Asset contract: 18 upright yaw views, east first, clockwise in a 6×3 atlas.
const FRAME_COUNT: usize = 18;
const ATLAS_COLUMNS: usize = 6;
const FRAME_SIZE: usize = 256;

pub struct UnitMotion {
    motion: Motion,
    destination: Pose,
    last_pose: Option<Pose>,
    last_frame: Option<usize>,
    node: Element,
    atlas_viewport: Element,
    atlas_image: Element,
    atlas_load: Closure<dyn FnMut(web_sys::Event)>,
}

impl Drop for UnitMotion {
    fn drop(&mut self) {
        // Reset/removal may happen before the image request finishes.
        let _ = self
            .atlas_image
            .remove_event_listener_with_callback("load", self.atlas_load.as_ref().unchecked_ref());
    }
}

pub fn sprite(unit: &Unit) -> String {
    format!("/sprites/{}.png", unit.role().id)
}
fn points(x: f64, y: f64, radius: f64) -> String {
    (0..6)
        .map(|i| {
            let a = (60.0 * i as f64 - 30.0).to_radians();
            format!("{},{}", x + radius * a.cos(), y + radius * a.sin())
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn polygon(hex: Hex, radius: f64) -> String {
    let (x, y) = position(hex);
    format!(r#"<polygon points="{}"/>"#, points(x, y, radius))
}
pub fn base() -> String {
    let (tobruk_x, tobruk_y) = position(TOBRUK);
    let coast = COAST
        .iter()
        .map(|(x, y)| format!("{x},{y}"))
        .collect::<Vec<_>>()
        .join(" ");
    let grid: String = hexes().into_iter().map(|h| polygon(h, 41.57)).collect();
    let ridges:String=hexes().into_iter().filter(|h|terrain(*h)=="ridge").map(|h|{
        let (x,y)=position(h); format!(r##"<g transform="translate({},{})" opacity=".5" fill="#b8a478" stroke="#9b885e" stroke-width=".9"><path d="M0 20L14 0L27 20M17 23L34-5L52 23"/><path d="M14 0L17 14L23 13M34-5L36 13L42 10" fill="none"/></g>"##,x-25.0,y-8.0)
    }).collect();
    format!(
        r##"<svg class="battle-map" viewBox="0 0 1120 660" aria-label="Interactive tactical map of the Tobruk frontier" xmlns="http://www.w3.org/2000/svg">
    <defs><pattern id="sand" width="110" height="90" patternUnits="userSpaceOnUse"><path d="M10 20l2 1m40 30l2-1m37 25l1 1M75 12l2 0M23 70l1-2" stroke="#a58f64" stroke-width="1.2" opacity=".34"/><path d="M35 38q9-4 19-1m23 17q8-4 14-1" stroke="#c2ac7d" stroke-width=".6" fill="none" opacity=".35"/></pattern>
    <pattern id="waves" width="55" height="35" patternUnits="userSpaceOnUse"><path d="M4 18q7-3 15 0t15 0" stroke="#c6d2cc" stroke-width=".7" fill="none" opacity=".3"/></pattern>
    <filter id="paper"><feTurbulence type="fractalNoise" baseFrequency=".64" numOctaves="3" stitchTiles="stitch"/><feColorMatrix type="saturate" values="0"/><feComponentTransfer><feFuncA type="linear" slope=".055"/></feComponentTransfer><feBlend in="SourceGraphic" mode="multiply"/></filter>
    <filter id="unit-shadow" x="-40%" y="-40%" width="180%" height="200%"><feDropShadow dx="0" dy="3" stdDeviation="3" flood-color="#473d29" flood-opacity=".3"/></filter><clipPath id="land"><polygon points="0,660 0,173 {coast} 1120,660"/></clipPath></defs>
    <rect width="1120" height="660" fill="#a8bfc0"/><rect width="1120" height="660" fill="url(#waves)"/><polygon points="0,660 0,173 {coast} 1120,660" fill="#e2d3af"/>
    <g clip-path="url(#land)"><path d="M-70 370Q140 230 390 370T790 450T1180 440M-80 390Q140 250 390 390T790 470T1180 460M-80 410Q140 270 390 410T790 490T1180 480" stroke="#c4b285" stroke-width="1.1" opacity=".4" fill="none"/><path d="M-100 550Q200 420 360 540T720 560T1150 610M-100 560Q200 430 360 550T720 570T1150 620M-100 570Q200 440 360 560T720 580T1150 630" stroke="#c4b285" stroke-width=".8" opacity=".4" fill="none"/>
    <rect width="1120" height="660" fill="url(#sand)"/><g class="hex-grid">{grid}</g>
    <path d="M-10 219L116 226L187 220L296 235L366 236L441 270L517 283L595 314L664 350L723 351L790 413L865 413L944 405L1042 401L1130 375" stroke="#ae9670" stroke-width="5" fill="none" opacity=".65"/><path d="M-10 219L116 226L187 220L296 235L366 236L441 270L517 283L595 314L664 350L723 351L790 413L865 413L944 405L1042 401L1130 375" stroke="#f3e4c1" stroke-width="2.5" fill="none"/>
    <path d="M296 235Q310 305 330 350T440 479L656 479L764 541L1016 479" stroke="#ae9670" stroke-width="1.7" stroke-dasharray="5 5" fill="none" opacity=".6"/><path d="M855 338L850 660" stroke="#97856b" stroke-width="1.4" stroke-dasharray="8 6" opacity=".65"/>{ridges}</g>
    <polyline points="{coast}" stroke="#f6e8c6" stroke-width="5" fill="none"/><polyline points="{coast}" stroke="#8fa9a7" stroke-width="1.2" fill="none"/><rect width="1120" height="660" fill="transparent" filter="url(#paper)" opacity=".45" pointer-events="none"/>
    <g class="map-lettering" pointer-events="none"><text x="576" y="128" class="sea-name" text-anchor="middle" transform="rotate(8 576 128)">Mediterranean Sea</text><text x="80" y="70" class="region-name">CYRENAICA</text><text x="80" y="90" class="region-caption">Eastern Libya · Western Egypt</text><text x="180" y="564" class="desert-name">L I B Y A N   D E S E R T</text><text x="819" y="613" class="country-name" transform="rotate(-90 819 613)">LIBYA</text><text x="879" y="613" class="country-name" transform="rotate(-90 879 613)">EGYPT</text><text x="448" y="291" class="road-name" transform="rotate(21 448 291)">Via Balbia</text>
    <g transform="translate(297 190)"><text text-anchor="middle" class="town-name">Tobruk</text><text y="15" text-anchor="middle" class="objective-label">OBJECTIVE</text></g><g transform="translate(652 305)"><circle cy="14" r="3"/><text text-anchor="middle" class="town-name">Bardia</text></g><g transform="translate(842 386)"><circle cx="-15" cy="-4" r="3"/><text class="town-name">Sollum</text></g><g transform="translate(493 491)"><text class="small-town">Sidi Rezegh</text></g><g transform="translate(917 461)"><text class="small-town">Frontier depot</text></g><g transform="translate(1016 479)"><rect x="-12" y="-10" width="24" height="20" fill="#f1e5c6" stroke="#68745a"/><path d="M-12-10L0-18L12-10M-5 10V0H5V10" fill="none" stroke="#68745a" stroke-width="2"/></g></g>
    <g class="supply-overlay"></g><g class="move-targets"></g><g pointer-events="none"><circle cx="{tobruk_x}" cy="{tobruk_y}" r="33" fill="none" stroke="#a57e3b" stroke-width="2" stroke-dasharray="4 4"/></g><g class="units"></g>
    <g transform="translate(1030 91)" class="compass" pointer-events="none"><text y="-32" text-anchor="middle">N</text><path d="M0-22L7 0L0 22L-7 0Z" fill="#4c6665"/><path d="M-22 0L0-7L22 0L0 7Z" fill="#d5dfd5"/><circle r="3" fill="#4c6665"/></g></svg>"##
    )
}

pub fn update(app: &mut App) {
    let svg = document().query_selector(".battle-map").unwrap().unwrap();
    svg.set_attribute(
        "class",
        if app.grid {
            "battle-map"
        } else {
            "battle-map no-grid"
        },
    )
    .unwrap();
    let (cx, cy) = app
        .selected()
        .map(|u| position(u.hex()))
        .unwrap_or((560.0, 330.0));
    let (w, h) = (1120.0 / app.zoom, 660.0 / app.zoom);
    svg.set_attribute(
        "viewBox",
        &format!(
            "{} {} {w} {h}",
            (cx - w / 2.0).clamp(0.0, 1120.0 - w),
            (cy - h / 2.0).clamp(0.0, 660.0 - h)
        ),
    )
    .unwrap();
    let supplies = if app.supplies {
        all_hexes()
            .into_iter()
            .filter(|h| {
                distance(*h, DEPOT) <= 2
                    || app.game.units.iter().any(|u| {
                        u.side == "allied"
                            && u.hp > 0
                            && u.role().support == Support::Supply
                            && distance(*h, u.hex()) <= 2
                    })
            })
            .map(|h| polygon(h, 40.0))
            .collect()
    } else {
        String::new()
    };
    svg.query_selector(".supply-overlay")
        .unwrap()
        .unwrap()
        .set_inner_html(&supplies);
    let mut moves: Vec<_> = if app.mode == "move" {
        reachable(&app.game, app.selected()).into_iter().collect()
    } else if app.mode == "deploy" && app.game.outcome.is_none() {
        app.deployment
            .as_ref()
            .and_then(|d| role(&d.role_id))
            .map(|role| {
                all_hexes()
                    .into_iter()
                    .filter(|&h| passable(role, h) && unit_at(&app.game, h).is_none())
                    .map(|h| (h, 0))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        vec![]
    };
    moves.sort_by_key(|(hex, _)| (hex.r, hex.c));
    let action = if app.mode == "deploy" {
        "Deploy at"
    } else {
        "Move to"
    };
    let targets:String=moves.into_iter().map(|(hex,cost)|{let (x,y)=position(hex);let label=if app.mode=="deploy"{"+".into()}else{cost.to_string()};format!(r#"<g role="button" tabindex="0" data-hex="{},{}" aria-label="{action} hex {}:{}, {cost} CP">{}<text x="{x}" y="{}" text-anchor="middle">{label}</text></g>"#,hex.c,hex.r,hex.c+1,hex.r+1,polygon(hex,39.0),y+4.0)}).collect();
    svg.query_selector(".move-targets")
        .unwrap()
        .unwrap()
        .set_inner_html(&targets);
    let layer = svg.query_selector(".units").unwrap().unwrap();
    let time = now();
    let units = app.game.units.clone();
    app.motions.retain(|id, entry| {
        let alive = units.iter().any(|u| u.id == *id && u.hp > 0);
        if !alive {
            entry.node.remove();
        }
        alive
    });
    // A reset discards presentation state; remove its old nodes before recreating keys.
    if app.motions.is_empty() {
        layer.set_inner_html("");
    }
    for u in units.iter().filter(|u| u.hp > 0) {
        let selected = u.id == app.selected;
        let target = (app.mode == "attack" && can_attack(&app.game, app.selected(), Some(u)))
            || (app.mode == "support" && can_support(&app.game, app.selected(), Some(u)));
        let (x, y) = position(u.hex());
        let portrait = escape(&app.inventory.portrait(u));
        let atlas_path = app.inventory.atlas(u);
        let entry=app.motions.entry(u.id.clone()).or_insert_with(||{
            let node=document().create_element_ns(Some("http://www.w3.org/2000/svg"),"g").unwrap();
            node.set_attribute("data-unit",&u.id).unwrap();node.set_attribute("role","button").unwrap();node.set_attribute("tabindex","0").unwrap();
            let atlas_id = format!("unit-atlas-{}", u.id);
            let atlas_width = ATLAS_COLUMNS * FRAME_SIZE;
            let atlas_height = FRAME_COUNT / ATLAS_COLUMNS * FRAME_SIZE;
            node.set_inner_html(&format!(r##"<title></title><ellipse class="selection-halo" rx="39" ry="29" cy="3"/><path class="entrenchment" d="M-34 8L-34 20L-18 28H18L34 20V8" fill="none" stroke="#605a3d" stroke-width="4"/><defs><pattern id="{atlas_id}" class="unit-atlas" patternUnits="userSpaceOnUse" x="-30" y="-37" width="60" height="60" viewBox="0 0 {FRAME_SIZE} {FRAME_SIZE}" overflow="hidden"><image width="{atlas_width}" height="{atlas_height}" preserveAspectRatio="none"/></pattern></defs><g class="unit-hover" filter="url(#unit-shadow)"><image href="{portrait}" x="-39" y="-37" width="78" height="60" class="unit-sprite"/><g class="unit-frames" visibility="hidden"><rect class="frame-from" x="-30" y="-37" width="60" height="60" fill="url(#{atlas_id})"/></g></g><rect class="counter" x="-26" y="16" width="52" height="19" rx="3"/><text class="counter-text" x="0" y="29" text-anchor="middle">{}</text><rect x="-23" y="35" width="46" height="3" rx="1" fill="#685b45" opacity=".3"/><rect class="health" x="-23" y="35" height="3" rx="1" fill="{}"/><circle class="target-ring" r="32" fill="none" stroke="#a04c38" stroke-width="2" stroke-dasharray="4 3"/>"##,escape(&u.short),if u.side=="allied"{"#546a46"}else{"#a15c45"}));
            // A square pattern clips exactly one cell. Keeping the full sheet in
            // defs also keeps its offscreen pixels out of unit focus/hit bounds.
            let atlas_viewport = node.query_selector(".unit-atlas").unwrap().unwrap();
            let atlas_image = atlas_viewport.query_selector("image").unwrap().unwrap();
            let fallback = node.query_selector(".unit-sprite").unwrap().unwrap();
            let loaded_frame = node.query_selector(".unit-frames").unwrap().unwrap();
            let atlas_load = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                loaded_frame.set_attribute("visibility", "visible").unwrap();
                fallback.set_attribute("visibility", "hidden").unwrap();
            });
            atlas_image.add_event_listener_with_callback("load", atlas_load.as_ref().unchecked_ref()).unwrap();
            // Missing/loading atlases leave the original upright sprite visible.
            atlas_image.set_attribute("href", &atlas_path).unwrap();
            layer.append_child(&node).unwrap();
            let destination=Pose{x,y,heading:160.0};UnitMotion{motion:Motion::new(destination),destination,last_pose:None,last_frame:None,node,atlas_viewport,atlas_image,atlas_load}
        });
        if x != entry.destination.x || y != entry.destination.y {
            let heading = (y - entry.destination.y)
                .atan2(x - entry.destination.x)
                .to_degrees();
            // Stop on a real view so stationary units never remain double-exposed.
            let heading = directional_frame(heading, FRAME_COUNT) as f64 * 20.0;
            entry.destination = Pose { x, y, heading };
            entry.motion.retarget(entry.destination, time);
        }
        entry
            .node
            .set_attribute(
                "class",
                &format!(
                    "map-unit {} {} {}",
                    u.side,
                    if selected { "selected" } else { "" },
                    if target { "targetable" } else { "" }
                ),
            )
            .unwrap();
        entry
            .node
            .set_attribute(
                "aria-label",
                &format!(
                    "{}, {}, strength {}%",
                    u.name,
                    if u.side == "allied" {
                        "Commonwealth"
                    } else {
                        "Axis"
                    },
                    u.hp
                ),
            )
            .unwrap();
        entry
            .node
            .set_attribute("aria-pressed", if selected { "true" } else { "false" })
            .unwrap();
        entry
            .node
            .query_selector("title")
            .unwrap()
            .unwrap()
            .set_text_content(Some(&format!(
                "{} · {}% strength · {} CP{}",
                u.name,
                u.hp,
                u.cp,
                if u.entrenched { " · Dug in" } else { "" }
            )));
        entry
            .node
            .query_selector(".health")
            .unwrap()
            .unwrap()
            .set_attribute("width", &(46.0 * u.hp as f64 / 100.0).to_string())
            .unwrap();
        entry
            .node
            .query_selector(".entrenchment")
            .unwrap()
            .unwrap()
            .set_attribute(
                "visibility",
                if u.entrenched { "visible" } else { "hidden" },
            )
            .unwrap();
        entry
            .node
            .query_selector(".target-ring")
            .unwrap()
            .unwrap()
            .set_attribute("visibility", if target { "visible" } else { "hidden" })
            .unwrap();
    }
    if animate(app) {
        crate::request_frame();
    }
}

pub fn animate(app: &mut App) -> bool {
    let reduced = web_sys::window()
        .unwrap()
        .match_media("(prefers-reduced-motion: reduce)")
        .unwrap()
        .is_some_and(|m| m.matches());
    let time = now();
    let mut active = false;
    for entry in app.motions.values_mut() {
        let pose = entry.motion.sample(time, reduced);
        active |= entry.motion.is_active(time);
        if entry.last_pose == Some(pose) {
            continue;
        }
        entry.last_pose = Some(pose);
        entry
            .node
            .set_attribute("transform", &format!("translate({} {})", pose.x, pose.y))
            .unwrap();
        // One fully opaque view avoids double silhouettes during a fast turn.
        let frame = directional_frame(pose.heading, FRAME_COUNT);
        if entry.last_frame != Some(frame) {
            entry.last_frame = Some(frame);
            let x = frame % ATLAS_COLUMNS * FRAME_SIZE;
            let y = frame / ATLAS_COLUMNS * FRAME_SIZE;
            entry
                .atlas_viewport
                .set_attribute("viewBox", &format!("{x} {y} {FRAME_SIZE} {FRAME_SIZE}"))
                .unwrap();
        }
    }
    active
}
