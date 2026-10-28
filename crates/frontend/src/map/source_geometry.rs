//! Source-aligned display geometry; deliberately separate from network links.
use crate::view::escape;
use cna_map::{geometry::BoardPoint, hex::HexsideId, viewport::{ScreenPoint, Viewport}};
use serde::Deserialize;
use std::{fmt::Write, sync::OnceLock};

#[derive(Deserialize)]
struct ObservationRef {
    side: HexsideId,
}

#[derive(Deserialize)]
struct Trace {
    id: String,
    depiction: String,
    observation_refs: Vec<ObservationRef>,
    quadratics: Vec<[[f64; 2]; 3]>,
    bounds: [f64; 4],
}

#[derive(Deserialize)]
struct Paths {
    paths: Vec<Trace>,
}

#[derive(Deserialize)]
struct Outline {
    id: String,
    rings: Vec<Vec<[f64; 2]>>,
    bounds: [f64; 4],
}

#[derive(Deserialize)]
struct Water {
    outlines: Vec<Outline>,
}

fn paths() -> &'static Paths {
    static PATHS: OnceLock<Paths> = OnceLock::new();
    PATHS.get_or_init(|| serde_json::from_str(include_str!("../../../../data/map/source-paths.json")).expect("verified source paths"))
}

fn water() -> &'static Water {
    static WATER: OnceLock<Water> = OnceLock::new();
    WATER.get_or_init(|| serde_json::from_str(include_str!("../../../../data/map/source-water.json")).expect("verified water outlines"))
}

pub(super) fn has_trace(side: HexsideId, depiction: &str) -> bool {
    paths().paths.iter().any(|p| p.depiction == depiction && p.observation_refs.iter().any(|r| r.side == side))
}

pub(super) fn render(viewport: Viewport, water_layer: bool) -> String {
    let [width, height] = viewport.css_size();
    let low = viewport.to_board(ScreenPoint { x: -width * 0.25, y: -height * 0.25 });
    let high = viewport.to_board(ScreenPoint { x: width * 1.25, y: height * 1.25 });
    let visible = |b: [f64; 4]| b[0] <= high.x && b[1] <= high.y && b[2] >= low.x && b[3] >= low.y;
    let origin = viewport.to_screen(BoardPoint { x: 0.0, y: 0.0 });
    let mut html = format!("<g class=\"source-geometry\" transform=\"translate({:.2} {:.2}) scale({})\">", origin.x, origin.y, viewport.zoom());
    if water_layer {
        for outline in water().outlines.iter().filter(|p| visible(p.bounds)) {
            write!(html, "<path class=\"source-water\" data-source-id=\"{}\" fill-rule=\"evenodd\" d=\"", escape(&outline.id)).unwrap();
            for ring in &outline.rings {
                for (i, [x, y]) in ring.iter().enumerate() {
                    write!(html, "{}{x},{y}", if i == 0 { "M" } else { "L" }).unwrap();
                }
                html.push('Z');
            }
            html.push_str("\"><title>Source water outline · provisional visual evidence, not a river or coast classification</title></path>");
        }
    } else {
        for trace in paths().paths.iter().filter(|p| visible(p.bounds)) {
            let mut d = String::new();
            for (i, [a, b, c]) in trace.quadratics.iter().enumerate() {
                if i == 0 { write!(d, "M{},{}", a[0], a[1]).unwrap(); }
                write!(d, "Q{},{} {},{}", b[0], b[1], c[0], c[1]).unwrap();
            }
            write!(html, "<path class=\"source-route {}\" data-source-id=\"{}\" d=\"{d}\"><title>Source-aligned {} · provisional display trace, not movement authority</title></path>", escape(&trace.depiction), escape(&trace.id), escape(&trace.depiction)).unwrap();
            let detail = if trace.depiction == "road" { "road-fill" } else { "railroad-ties" };
            write!(html, "<path class=\"source-route {detail}\" d=\"{d}\"/>").unwrap();
        }
    }
    html.push_str("</g>");
    html
}
