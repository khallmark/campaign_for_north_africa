//! Research geometry and evidence overlay, independent of counter placement.
use super::art::Artwork;
use crate::{document, view::escape};
use cna_map::{
    geometry::{center, corners, edge, hexes},
    hex::HexId,
    model::{model, Coverage},
    terrain::{MapRecord, Observation, TransportDepiction},
    viewport::Viewport,
};
use std::{collections::BTreeMap, fmt::Write};
use web_sys::Element;

pub(super) struct Geography {
    svg: Element,
    ground: Element,
    inspector: Element,
    records: Vec<Vec<MapRecord>>,
    artwork: Artwork,
    last_render: Option<(Viewport, [bool; 3], Option<HexId>)>,
    ground_viewport: Option<Viewport>,
    pub original_art: bool,
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
        let records: Vec<_> = model()
            .surveys
            .iter()
            .map(|survey| survey.records())
            .collect();
        let artwork = Artwork::new(&records);
        let ground = document()
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
            .unwrap();
        ground.set_attribute("class", "terrain-art-board").unwrap();
        ground.set_attribute("aria-hidden", "true").unwrap();
        svg.parent_node()
            .unwrap()
            .insert_before(&ground, Some(&svg))
            .unwrap();
        Self {
            svg,
            ground,
            artwork,
            records,
            last_render: None,
            ground_viewport: None,
            original_art: true,
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
                    MapRecord::Hexside {
                        side,
                        record,
                        depiction,
                    } => {
                        let other = side.hex().neighbor(side.direction()).unwrap();
                        let depiction_label = match depiction {
                            Some(TransportDepiction::UnfinishedRoad) => " · Unfinished road depiction — schematic, not an operational link",
                            Some(TransportDepiction::UnfinishedRailroad) => " · Unfinished railroad depiction — schematic, not an operational link",
                            None => "",
                        };
                        (
                            hex == side.hex() || hex == other,
                            serde_json::to_value(&record.observation).unwrap(),
                            format!("Side {} ↔ {other}{depiction_label}", side.hex()),
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

    pub fn render(&mut self, viewport: Viewport) {
        let state = (
            viewport,
            [self.original_art, self.grid, self.features],
            self.selected,
        );
        let [width, height] = viewport.css_size();
        // Retain a quarter-viewport overscan on each side. Rebuild only when
        // the visible rectangle leaves that buffer, not on every pointer move.
        let ground_covers_viewport = self.ground_viewport.is_some_and(|old| {
            old.zoom() == viewport.zoom()
                && old.css_size() == viewport.css_size()
                && old.device_pixel_ratio() == viewport.device_pixel_ratio()
                && (old.center().x - viewport.center().x).abs() * viewport.zoom() <= width * 0.25
                && (old.center().y - viewport.center().y).abs() * viewport.zoom() <= height * 0.25
        });
        if self.original_art && !ground_covers_viewport {
            self.ground
                .set_inner_html(&self.artwork.render_ground(viewport));
            self.ground_viewport = Some(viewport);
            // Bound the SVG raster surface to the screen, not the 66MP board.
            self.ground
                .set_attribute("width", &(width * 1.5).to_string())
                .unwrap();
            self.ground
                .set_attribute("height", &(height * 1.5).to_string())
                .unwrap();
            self.ground
                .set_attribute(
                    "viewBox",
                    &format!(
                        "{} {} {} {}",
                        viewport.center().x - width * 0.75 / viewport.zoom(),
                        viewport.center().y - height * 0.75 / viewport.zoom(),
                        width * 1.5 / viewport.zoom(),
                        height * 1.5 / viewport.zoom()
                    ),
                )
                .unwrap();
        } else if !self.original_art && self.ground_viewport.take().is_some() {
            self.ground.set_inner_html("");
        }
        if let Some(old) = self.ground_viewport {
            self.ground
                .set_attribute(
                    "style",
                    &format!(
                        "transform:translate({}px,{}px)",
                        (old.center().x - viewport.center().x) * viewport.zoom() - width * 0.25,
                        (old.center().y - viewport.center().y) * viewport.zoom() - height * 0.25
                    ),
                )
                .unwrap();
        }
        self.ground
            .set_attribute("display", if self.original_art { "inline" } else { "none" })
            .unwrap();
        self.svg
            .set_attribute(
                "data-original-art",
                if self.original_art { "true" } else { "false" },
            )
            .unwrap();
        // Keep evidence and labels at their painted coordinates during a pan.
        // A CSS translation moves the buffered layer; zoom or state changes
        // repaint so screen-space strokes and labels retain their intended size.
        if let Some((old, flags, selected)) = self.last_render {
            let dx = (old.center().x - viewport.center().x) * viewport.zoom();
            let dy = (old.center().y - viewport.center().y) * viewport.zoom();
            if old.zoom() == viewport.zoom()
                && old.css_size() == viewport.css_size()
                && flags == state.1
                && selected == self.selected
                && dx.abs() <= width * 0.25
                && dy.abs() <= height * 0.25
            {
                self.svg
                    .set_attribute(
                        "style",
                        &format!(
                            "width:{}px;height:{}px;transform:translate({}px,{}px)",
                            width * 1.5,
                            height * 1.5,
                            dx - width * 0.25,
                            dy - height * 0.25
                        ),
                    )
                    .unwrap();
                return;
            }
        }
        self.svg
            .set_attribute(
                "viewBox",
                &format!(
                    "{} {} {} {}",
                    -width * 0.25,
                    -height * 0.25,
                    width * 1.5,
                    height * 1.5
                ),
            )
            .unwrap();
        self.svg
            .set_attribute(
                "style",
                &format!(
                    "width:{}px;height:{}px;transform:translate({}px,{}px)",
                    width * 1.5,
                    height * 1.5,
                    -width * 0.25,
                    -height * 0.25
                ),
            )
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
            c.x > -width * 0.25 - margin
                && c.x < width * 1.25 + margin
                && c.y > -height * 0.25 - margin
                && c.y < height * 1.25 + margin
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
        let art_locations = if self.original_art {
            self.artwork.render_points(viewport)
        } else {
            String::new()
        };
        if self.features {
            let mut links: BTreeMap<&str, String> = BTreeMap::new();
            let mut railroads = String::new();
            let mut construction = String::new();
            let mut locations = String::new();
            let mut water: BTreeMap<&str, String> = BTreeMap::new();
            for (&hex, examination) in &model().examinations {
                if visible(hex)
                    && matches!(examination.surface.as_str(), "water" | "mixed_water_land")
                {
                    if viewport.zoom() < 0.65 {
                        write!(
                            water.entry(&examination.surface).or_default(),
                            "M{}Z",
                            polygon(hex).replace(' ', "L")
                        )
                        .unwrap();
                        continue;
                    }
                    write!(
                        html,
                        "<polygon class=\"geography-water {}\" data-hex=\"{hex}\" points=\"{}\"/>",
                        examination.surface,
                        polygon(hex)
                    )
                    .unwrap();
                }
            }
            for (class, path) in water {
                write!(
                    html,
                    "<path class=\"geography-water {class}\" d=\"{path}\"/>"
                )
                .unwrap();
            }
            for records in &self.records {
                for assignment in records {
                    match assignment {
                        MapRecord::Terrain { hex, record }
                            if visible(*hex) && !self.original_art =>
                        {
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
                        MapRecord::Hexside {
                            side,
                            record,
                            depiction,
                        } if visible(side.hex()) => {
                            if let Some(depiction) = depiction {
                                // Source-backed depictions remain unresolved. These
                                // center-to-center strokes do not create network links.
                                let (class, label) = match depiction {
                                    TransportDepiction::UnfinishedRoad => {
                                        ("unfinished-road", "Unfinished road")
                                    }
                                    TransportDepiction::UnfinishedRailroad => {
                                        ("unfinished-railroad", "Unfinished railroad")
                                    }
                                };
                                let other = side.hex().neighbor(side.direction()).unwrap();
                                let a = viewport.to_screen(center(side.hex()));
                                let b = viewport.to_screen(center(other));
                                let path = format!("M{:.2},{:.2}L{:.2},{:.2}", a.x, a.y, b.x, b.y);
                                write!(construction, "<path class=\"geography-construction {class}\" data-from=\"{}\" data-to=\"{other}\" d=\"{path}\"><title>{label} depiction · unresolved · schematic, not an operational link</title></path>", side.hex()).unwrap();
                                if matches!(depiction, TransportDepiction::UnfinishedRailroad) {
                                    write!(construction, "<path class=\"geography-construction construction-ties\" d=\"{path}\"/>").unwrap();
                                }
                                continue;
                            }
                            let value = match &record.observation {
                                Observation::Provisional { value }
                                | Observation::Reviewed { value } => value,
                                Observation::Unresolved { .. } if viewport.zoom() >= 0.65 => {
                                    let (a, b) = edge(*side);
                                    let a = viewport.to_screen(a);
                                    let b = viewport.to_screen(b);
                                    write!(
                                        links.entry("unresolved").or_default(),
                                        "M{:.2},{:.2}L{:.2},{:.2}",
                                        a.x,
                                        a.y,
                                        b.x,
                                        b.y
                                    )
                                    .unwrap();
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
                                links.entry(class).or_default()
                            };
                            write!(layer, "M{:.2},{:.2}L{:.2},{:.2}", a.x, a.y, b.x, b.y).unwrap();
                        }
                        MapRecord::Point { hex, record } if visible(*hex) => {
                            let c = viewport.to_screen(center(*hex));
                            let unresolved =
                                matches!(record.observation, Observation::Unresolved { .. });
                            if self.original_art && !unresolved {
                                continue;
                            }
                            write!(locations, "<circle class=\"geography-location{}\" cx=\"{:.2}\" cy=\"{:.2}\" r=\"4\"/>", if unresolved {" unresolved"} else {""}, c.x,c.y).unwrap();
                        }
                        _ => {}
                    }
                }
            }
            // Terrain must not tint routes; the wider road stroke must not hide
            // a coincident railroad. Keep both visible regardless of survey order.
            for (class, path) in links {
                write!(
                    html,
                    "<path class=\"geography-link {class}\" d=\"{path}\"/>"
                )
                .unwrap();
            }
            if !railroads.is_empty() {
                write!(html, "<path class=\"geography-link railroad\" d=\"{railroads}\"/><path class=\"geography-link railroad railroad-ties\" d=\"{railroads}\"/>").unwrap();
            }
            html.push_str(&construction);
            html.push_str(&locations);
            html.push_str(&art_locations);
        }
        // Fine grid/IDs appear only when readable; full inventory still exists at fit.
        if self.grid && viewport.zoom() >= 0.22 {
            let mut label_centers: Vec<cna_map::viewport::ScreenPoint> = Vec::new();
            let mut grid_path = String::new();
            for &hex in hexes().iter().filter(|&&h| visible(h)) {
                if viewport.zoom() < 0.65 {
                    write!(grid_path, "M{}Z", polygon(hex).replace(' ', "L")).unwrap();
                    continue;
                }
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
            if !grid_path.is_empty() {
                write!(html, "<path class=geography-hex d=\"{grid_path}\"/>").unwrap();
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
        self.last_render = Some(state);
    }
}
