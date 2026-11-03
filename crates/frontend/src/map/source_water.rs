//! Source water, dry-wadi and scarp shapes retain widths without movement rules.
use crate::view::escape;
use cna_map::{
    geometry::BoardPoint,
    viewport::{ScreenPoint, Viewport},
};
use serde::Deserialize;
use std::{fmt::Write, sync::OnceLock};

#[derive(Deserialize)]
struct Outline {
    id: String,
    kind: String,
    rings: Vec<Vec<[f64; 2]>>,
    bounds: [f64; 4],
}

#[derive(Deserialize)]
struct Water {
    source_ref: String,
    outlines: Vec<Outline>,
}

pub(super) fn render(viewport: Viewport) -> String {
    static WATER: OnceLock<Water> = OnceLock::new();
    let water = WATER.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../data/map/source-water.json"))
            .expect("verified source water outlines")
    });
    let [width, height] = viewport.css_size();
    let low = viewport.to_board(ScreenPoint {
        x: -width * 0.25,
        y: -height * 0.25,
    });
    let high = viewport.to_board(ScreenPoint {
        x: width * 1.25,
        y: height * 1.25,
    });
    let origin = viewport.to_screen(BoardPoint { x: 0.0, y: 0.0 });
    let mut html = format!(
        r#"<g class="source-water-layer" data-source-ref="{}" transform="translate({:.2} {:.2}) scale({})"><title>Source water, dry-wadi and escarpment ink · provisional display evidence, not a river classification or movement rule. Printed scarp faces retained; Up/Down unresolved. Source overprint gaps retained.</title>"#,
        escape(&water.source_ref),
        origin.x,
        origin.y,
        viewport.zoom()
    );
    for outline in water.outlines.iter().filter(|outline| {
        let [left, top, right, bottom] = outline.bounds;
        left <= high.x && top <= high.y && right >= low.x && bottom >= low.y
    }) {
        write!(
            html,
            r#"<path class="source-{}" data-source-id="{}" fill-rule="evenodd" d=""#,
            escape(&outline.kind),
            escape(&outline.id)
        )
        .unwrap();
        for ring in &outline.rings {
            for (i, [x, y]) in ring.iter().enumerate() {
                write!(html, "{}{x},{y}", if i == 0 { "M" } else { "L" }).unwrap();
            }
            html.push('Z');
        }
        html.push_str("\"/>");
    }
    html.push_str("</g>");
    html
}
