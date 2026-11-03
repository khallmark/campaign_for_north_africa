//! Measured source curves are display evidence, never operational network links.
use crate::view::escape;
use cna_map::{
    geometry::BoardPoint,
    hex::HexsideId,
    terrain::HexsideFeature,
    viewport::{ScreenPoint, Viewport},
};
use serde::Deserialize;
use std::{fmt::Write, sync::OnceLock};

#[derive(Deserialize)]
struct Trace {
    id: String,
    depiction: String,
    observation_refs: Vec<ObservationRef>,
    quadratics: Vec<[[f64; 2]; 3]>,
    bounds: [f64; 4],
}

#[derive(Deserialize)]
struct ObservationRef {
    side: HexsideId,
}

#[derive(Deserialize)]
struct Routes {
    source_ref: String,
    source_sha256: String,
    paths: Vec<Trace>,
}

fn routes() -> &'static Routes {
    static ROUTES: OnceLock<Routes> = OnceLock::new();
    ROUTES.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../data/map/source-paths.json"))
            .expect("verified source route curves")
    })
}

pub(super) fn has_trace(side: HexsideId, feature: &HexsideFeature) -> bool {
    let kind = match feature {
        HexsideFeature::Road => "road",
        HexsideFeature::Railroad => "railroad",
        _ => return false,
    };
    routes().paths.iter().any(|trace| {
        trace.depiction == kind
            && trace
                .observation_refs
                .iter()
                .any(|reference| reference.side == side)
    })
}

pub(super) fn render(viewport: Viewport) -> String {
    let routes = routes();
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
        r#"<g class="source-route-layer" data-source-ref="{}" data-source-sha256="{}" transform="translate({:.2} {:.2}) scale({})"><title>Source road and railroad curves · provisional display evidence, not movement authority. Gaps and untraced portions remain; visual continuity does not establish section-seam adjacency.</title>"#,
        escape(&routes.source_ref),
        escape(&routes.source_sha256),
        origin.x,
        origin.y,
        viewport.zoom()
    );
    // Paint every road before every railroad. Source positions are never
    // displaced to create display lanes where their strokes run side by side.
    for kind in ["road", "railroad"] {
        for trace in routes.paths.iter().filter(|trace| {
            let [left, top, right, bottom] = trace.bounds;
            trace.depiction == kind
                && left <= high.x
                && top <= high.y
                && right >= low.x
                && bottom >= low.y
        }) {
            let mut path = String::new();
            for (index, [a, b, c]) in trace.quadratics.iter().enumerate() {
                if index == 0 {
                    write!(path, "M{},{}", a[0], a[1]).unwrap();
                }
                write!(path, "Q{},{} {},{}", b[0], b[1], c[0], c[1]).unwrap();
            }
            let detail = if kind == "road" {
                "road-fill"
            } else {
                "railroad-ties"
            };
            write!(html, r#"<g data-source-id="{}"><path class="geography-link source-route {kind}" d="{path}"/><path class="geography-link source-route {detail}" d="{path}"/></g>"#, escape(&trace.id)).unwrap();
        }
    }
    html.push_str("</g>");
    html
}
