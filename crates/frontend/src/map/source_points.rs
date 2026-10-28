//! Measured source glyphs are display evidence, not operational locations.
use crate::view::escape;
use cna_map::{
    geometry::BoardPoint,
    hex::HexId,
    model::{model, SourcePoint, SourcePointStatus, SourceSymbol},
    terrain::{Observation, PointFeature, Record},
    viewport::Viewport,
};
use std::fmt::Write;

pub(super) fn description(point: &SourcePoint) -> (&'static str, &'static str) {
    let symbol = match point.symbol {
        SourceSymbol::VillageBirDot => "Village/Bir source dot",
        SourceSymbol::Oasis => "Oasis source glyph",
        SourceSymbol::PortIcon => "Port source glyph",
    };
    let status = match point.status {
        SourcePointStatus::Provisional => "provisional",
        SourcePointStatus::Unresolved => "unresolved",
    };
    (symbol, status)
}

pub(super) fn replaces_marker(hex: HexId, record: &Record<PointFeature>) -> bool {
    model().examinations.get(&hex).is_some_and(|examination| {
        examination.points.iter().any(|point| {
            match &record.observation {
                // A measured glyph replaces only the same kind in this cell;
                // wells, airfields and other co-located facilities remain visible.
                Observation::Reviewed { value } | Observation::Provisional { value } => matches!(
                    (&point.symbol, value),
                    (SourceSymbol::Oasis, PointFeature::Oasis)
                        | (SourceSymbol::PortIcon, PointFeature::Port { .. })
                ),
                Observation::Unresolved { .. } => point.source_refs.iter().any(|reference| {
                    if reference.contains("#xywh=")
                        && record.provenance.source_refs.contains(reference)
                    {
                        return true;
                    }
                    let source = reference.split('#').next().unwrap();
                    let references = &record.provenance.source_refs;
                    point.symbol == SourceSymbol::PortIcon
                        && (references.contains(&format!(
                            "{source}#examination/{}/point/port_icon",
                            hex.section()
                        )) || (references.iter().any(|r| r.ends_with("#legend/Port"))
                            && references
                                .iter()
                                .any(|r| crop_contains(r, source, point.center))))
                }),
                Observation::Fixture { .. } => false,
            }
        })
    })
}

fn crop_contains(reference: &str, source: &str, point: [f64; 2]) -> bool {
    let Some((path, crop)) = reference.split_once("#xywh=") else {
        return false;
    };
    let Ok(values) = crop
        .split(',')
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
    else {
        return false;
    };
    let [x, y, width, height] = values.as_slice() else {
        return false;
    };
    path == source
        && point[0] >= *x
        && point[0] < x + width
        && point[1] >= *y
        && point[1] < y + height
}

pub(super) fn render(viewport: Viewport) -> String {
    if viewport.zoom() < 0.22 {
        return String::new();
    }
    let [width, height] = viewport.css_size();
    let mut html = String::new();
    for (hex, examination) in &model().examinations {
        for point in &examination.points {
            let [x, y] = point.center;
            let c = viewport.to_screen(BoardPoint { x, y });
            if c.x < -width * 0.25 - 150.0
                || c.x > width * 1.25 + 150.0
                || c.y < -height * 0.25 - 30.0
                || c.y > height * 1.25 + 30.0
            {
                continue;
            }
            let (symbol, status) = description(point);
            write!(html, r#"<g class="source-point {status}" data-hex="{hex}" transform="translate({:.2} {:.2})"><title>{symbol} · {status} · measured source position, not operational authority</title>"#, c.x, c.y).unwrap();
            match point.symbol {
                SourceSymbol::VillageBirDot => html.push_str(r#"<circle class="source-point-symbol" r="4"/>"#),
                SourceSymbol::Oasis => html.push_str(r#"<ellipse class="source-point-symbol" rx="7" ry="4"/><path class="source-point-symbol" d="M0 0V-10M-5-6Q0-12 5-6"/>"#),
                SourceSymbol::PortIcon => html.push_str(r#"<path class="source-point-symbol" d="M0-7V7M-5-3H5M-7 1Q-7 7 0 7Q7 7 7 1"/><circle class="source-point-symbol" cy="-9" r="2"/>"#),
            }
            if viewport.zoom() >= 0.65 {
                if let Some(label) = &point.label {
                    write!(
                        html,
                        r#"<text class="source-point-label" x="9" y="-9">{}</text>"#,
                        escape(label)
                    )
                    .unwrap();
                }
            }
            html.push_str("</g>");
        }
    }
    html
}
