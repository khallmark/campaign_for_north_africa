//! Research geometry and evidence overlay, independent of counter placement.
use crate::{document, view::escape};
use cna_map::{
    geometry::{center, corners, edge, hexes},
    hex::HexId,
    model::{model, Coverage},
    terrain::{MapRecord, Observation},
    viewport::Viewport,
};
use std::fmt::Write;
use web_sys::Element;

pub(super) struct Geography {
    svg: Element,
    inspector: Element,
    records: Vec<Vec<MapRecord>>,
    pub grid: bool,
    pub features: bool,
    pub inspecting: bool,
    pub selected: Option<HexId>,
}

impl Geography {
    pub fn new(root: &Element) -> Self {
        let svg = document()
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
            .unwrap();
        svg.set_attribute("class", "geography-overlay").unwrap();
        svg.set_attribute("aria-hidden", "true").unwrap();
        root.query_selector(".research-viewport")
            .unwrap()
            .unwrap()
            .append_child(&svg)
            .unwrap();
        let terrain_count: usize = model()
            .surveys
            .iter()
            .filter_map(|s| match &s.coverage {
                Coverage::Terrain { hexes, .. } => Some(hexes.len()),
                _ => None,
            })
            .sum();
        root.query_selector(".geography-coverage").unwrap().unwrap().set_text_content(Some(&format!(
            "{} source-grid hexes · {} raster examinations · {terrain_count} terrain observations · transcription incomplete · seams unresolved", hexes().len(), model().examinations.len()
        )));
        Self {
            svg,
            records: model()
                .surveys
                .iter()
                .map(|survey| survey.records())
                .collect(),
            inspector: root
                .query_selector(".geography-inspector")
                .unwrap()
                .unwrap(),
            grid: true,
            features: true,
            inspecting: false,
            selected: None,
        }
    }

    pub fn inspect(&mut self, hex: Option<HexId>) {
        self.selected = hex;
        let Some(hex) = hex else {
            self.inspector.set_inner_html("<strong>No source-grid hex here</strong><p>Boxes and outside-section space are not playable hexes. Try a section-local identity such as D3714.</p>");
            return;
        };
        let mut html = format!("<strong>{hex} · section-local identity</strong><p>Geometry: center inside source polygon. Playability unknown; printed identity and seams unresolved.</p>");
        if let Some(examination) = model().examinations.get(&hex) {
            write!(html, "<details class=geography-examination><summary>Raster examination · {}</summary><p>Automated evidence, not human-reviewed. Water includes sea and inland water; mixed cells are not automatically coast. Not detected does not mean absent.</p><p>Source crop xywh={:?}</p>", escape(&examination.surface), examination.crop).unwrap();
            for (family, status) in &examination.families {
                write!(html, "<p>{}: {}</p>", escape(family), escape(status)).unwrap();
            }
            html.push_str("</details>");
        }
        let mut terrain = false;
        let mut count = 0;
        for (survey, records) in model().surveys.iter().zip(&self.records) {
            for assignment in records {
                let (matches, observation, detail) = match assignment {
                    MapRecord::Terrain { hex: h, record } => {
                        if *h == hex {
                            terrain = true;
                        }
                        (
                            *h == hex,
                            serde_json::to_value(&record.observation).unwrap(),
                            "Terrain".to_string(),
                        )
                    }
                    MapRecord::Point { hex: h, record } => (
                        *h == hex,
                        serde_json::to_value(&record.observation).unwrap(),
                        "Location".to_string(),
                    ),
                    MapRecord::Hexside { side, record, .. } => {
                        let other = side.hex().neighbor(side.direction()).unwrap();
                        (
                            hex == side.hex() || hex == other,
                            serde_json::to_value(&record.observation).unwrap(),
                            format!("Side {} ↔ {other}", side.hex()),
                        )
                    }
                };
                if !matches {
                    continue;
                }
                count += 1;
                let status = observation["status"].as_str().unwrap();
                let value = observation
                    .get("value")
                    .map(|v| {
                        v.as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| v.to_string())
                    })
                    .unwrap_or_else(|| observation["source_label"].as_str().unwrap().to_string());
                write!(
                    html,
                    "<details><summary>{}: {} · {}</summary><p>{}: {}</p><p>{}</p><p>{} · {}</p>",
                    escape(&survey.label),
                    escape(&value),
                    status,
                    detail,
                    status,
                    escape(&survey.notes),
                    escape(&survey.provenance.reviewer),
                    escape(&survey.provenance.date)
                )
                .unwrap();
                for reference in &survey.provenance.source_refs {
                    let path = reference.split('#').next().unwrap();
                    write!(
                        html,
                        "<p class=geography-source>{}<br>SHA256 {}</p>",
                        escape(reference),
                        model().sources[path]
                    )
                    .unwrap();
                }
                html.push_str("</details>");
            }
        }
        if !terrain {
            html.push_str("<p><b>Terrain unmapped — not Clear.</b></p>");
        } else {
            html.push_str(
                "<p>Terrain transcription partial; additional features may be missing.</p>",
            );
        }
        if count == 0 {
            html.push_str("<p>No geography observations yet.</p>");
        }
        self.inspector.set_inner_html(&html);
    }

    pub fn render(&self, viewport: Viewport) {
        let [width, height] = viewport.css_size();
        self.svg
            .set_attribute("viewBox", &format!("0 0 {width} {height}"))
            .unwrap();
        self.svg
            .set_attribute(
                "data-detail",
                if viewport.zoom() >= 0.22 {
                    "true"
                } else {
                    "false"
                },
            )
            .unwrap();
        let visible = |hex| {
            let c = viewport.to_screen(center(hex));
            let margin = 90.0 * viewport.zoom();
            c.x > -margin && c.x < width + margin && c.y > -margin && c.y < height + margin
        };
        let polygon = |hex| {
            corners(hex)
                .iter()
                .map(|&p| {
                    let p = viewport.to_screen(p);
                    format!("{:.2},{:.2}", p.x, p.y)
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        let mut html = String::new();
        if self.features {
            let mut links = String::new();
            let mut railroads = String::new();
            let mut locations = String::new();
            for (&hex, examination) in &model().examinations {
                if visible(hex)
                    && matches!(examination.surface.as_str(), "water" | "mixed_water_land")
                {
                    write!(
                        html,
                        "<polygon class=\"geography-water {}\" data-hex=\"{hex}\" points=\"{}\"/>",
                        examination.surface,
                        polygon(hex)
                    )
                    .unwrap();
                }
            }
            for records in &self.records {
                for assignment in records {
                    match assignment {
                        MapRecord::Terrain { hex, record } if visible(*hex) => {
                            let color = match &record.observation {
                                Observation::Provisional { value }
                                | Observation::Reviewed { value } => match value {
                                    cna_map::terrain::Terrain::Desert => "#d9a92e",
                                    cna_map::terrain::Terrain::Mountain => "#806044",
                                    cna_map::terrain::Terrain::Delta => "#3d877b",
                                    cna_map::terrain::Terrain::SaltMarsh => "#9b83af",
                                    cna_map::terrain::Terrain::Clear => "#eee9d2",
                                    cna_map::terrain::Terrain::Gravel => "#b7a77a",
                                    cna_map::terrain::Terrain::HeavyVegetation
                                    | cna_map::terrain::Terrain::Swamp => "#629659",
                                    _ => "#ad8c58",
                                },
                                _ => "#a65e72",
                            };
                            write!(html, "<polygon class=geography-terrain data-hex=\"{hex}\" points=\"{}\" fill=\"{color}\"/>", polygon(*hex)).unwrap();
                        }
                        MapRecord::Hexside { side, record, .. } if visible(side.hex()) => {
                            let value = match &record.observation {
                                Observation::Provisional { value }
                                | Observation::Reviewed { value } => value,
                                Observation::Unresolved { .. } if viewport.zoom() >= 0.65 => {
                                    let (a, b) = edge(*side);
                                    let a = viewport.to_screen(a);
                                    let b = viewport.to_screen(b);
                                    write!(links, "<path class=\"geography-link unresolved\" d=\"M{:.2},{:.2}L{:.2},{:.2}\"/>",a.x,a.y,b.x,b.y).unwrap();
                                    continue;
                                }
                                _ => continue,
                            };
                            use cna_map::terrain::HexsideFeature::*;
                            let (a, b, class) =
                                if matches!(value, Road | Railroad | Track | Pipeline) {
                                    (
                                        center(side.hex()),
                                        center(side.hex().neighbor(side.direction()).unwrap()),
                                        match value {
                                            Road => "road",
                                            Railroad => "railroad",
                                            Track => "track",
                                            _ => "pipeline",
                                        },
                                    )
                                } else {
                                    let (a, b) = edge(*side);
                                    (a, b, "side")
                                };
                            let a = viewport.to_screen(a);
                            let b = viewport.to_screen(b);
                            let layer = if matches!(value, Railroad) {
                                &mut railroads
                            } else {
                                &mut links
                            };
                            write!(layer, "<path class=\"geography-link {class}\" d=\"M{:.2},{:.2}L{:.2},{:.2}\"/>",a.x,a.y,b.x,b.y).unwrap();
                            if matches!(value, Railroad) {
                                write!(layer, "<path class=\"geography-link railroad railroad-ties\" d=\"M{:.2},{:.2}L{:.2},{:.2}\"/>",a.x,a.y,b.x,b.y).unwrap();
                            }
                        }
                        MapRecord::Point { hex, record } if visible(*hex) => {
                            let c = viewport.to_screen(center(*hex));
                            let unresolved =
                                matches!(record.observation, Observation::Unresolved { .. });
                            write!(locations, "<circle class=\"geography-location{}\" cx=\"{:.2}\" cy=\"{:.2}\" r=\"4\"/>", if unresolved {" unresolved"} else {""}, c.x,c.y).unwrap();
                        }
                        _ => {}
                    }
                }
            }
            // Terrain must not tint routes; the wider road stroke must not hide
            // a coincident railroad. Keep both visible regardless of survey order.
            html.push_str(&links);
            html.push_str(&railroads);
            html.push_str(&locations);
        }
        // Fine grid/IDs appear only when readable; full inventory still exists at fit.
        if self.grid && viewport.zoom() >= 0.22 {
            let mut label_centers: Vec<cna_map::viewport::ScreenPoint> = Vec::new();
            for &hex in hexes().iter().filter(|&&h| visible(h)) {
                write!(
                    html,
                    "<polygon class=geography-hex data-hex=\"{hex}\" points=\"{}\"/>",
                    polygon(hex)
                )
                .unwrap();
                if viewport.zoom() >= 0.65 {
                    let c = viewport.to_screen(center(hex));
                    // Co-located section IDs are separate observations, not aliases.
                    let overlapping = label_centers
                        .iter()
                        .filter(|p| (p.x - c.x).abs() < 40.0 && (p.y - c.y).abs() < 18.0)
                        .count();
                    label_centers.push(c);
                    write!(
                        html,
                        "<text class=geography-id x=\"{:.2}\" y=\"{:.2}\">{hex}</text>",
                        c.x,
                        c.y + 18.0 * viewport.zoom() + 20.0 * overlapping as f64
                    )
                    .unwrap();
                }
            }
        }
        if let Some(hex) = self.selected.filter(|&h| visible(h)) {
            write!(
                html,
                "<polygon class=geography-selected points=\"{}\"/>",
                polygon(hex)
            )
            .unwrap();
        }
        self.svg.set_inner_html(&html);
    }
}
