//! Original artwork depicts typed observations, never inferred geography.
use cna_map::{
    geometry::{center, corners, hexes},
    hex::HexId,
    terrain::{MapRecord, Observation},
    viewport::Viewport,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    sync::OnceLock,
};

fn manifest() -> &'static Value {
    static MANIFEST: OnceLock<Value> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../public/art/terrain-v1/manifest.json"
        ))
        .expect("verified terrain art manifest")
    })
}

fn known<T>(observation: &Observation<T>) -> Option<&T> {
    match observation {
        Observation::Reviewed { value } | Observation::Provisional { value } => Some(value),
        Observation::Unresolved { .. } | Observation::Fixture { .. } => None,
    }
}

pub(super) struct Artwork {
    terrain: BTreeMap<HexId, BTreeSet<String>>,
    points: BTreeMap<HexId, BTreeSet<String>>,
}

impl Artwork {
    pub fn new(records: &[Vec<MapRecord>]) -> Self {
        let mut art = Self {
            terrain: BTreeMap::new(),
            points: BTreeMap::new(),
        };
        for record in records.iter().flatten() {
            let (hex, names, ground) = match record {
                MapRecord::Terrain { hex, record } => {
                    let Some(value) = known(&record.observation) else {
                        continue;
                    };
                    let key = serde_json::to_value(value).unwrap();
                    let recipe = &manifest()["recipes"]["terrain"][key.as_str().unwrap()];
                    if let Some(surface) = recipe["surface"].as_str() {
                        (*hex, vec![surface.to_owned()], true)
                    } else {
                        // Minefield allegiance needs a viewing player, not a map record.
                        if recipe.get("requires").is_some() {
                            continue;
                        }
                        (
                            *hex,
                            recipe["stamps"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|value| value.as_str().unwrap().to_owned())
                                .collect(),
                            false,
                        )
                    }
                }
                MapRecord::Point { hex, record } => {
                    let Some(value) = known(&record.observation) else {
                        continue;
                    };
                    let value = serde_json::to_value(value).unwrap();
                    let recipe = &manifest()["recipes"]["point"][value["type"].as_str().unwrap()];
                    let name = if value["type"] == "fortification" {
                        recipe["level"][value["level"].as_str().unwrap()]
                            .as_str()
                            .unwrap()
                    } else {
                        recipe["asset"].as_str().unwrap()
                    };
                    (*hex, vec![name.to_owned()], false)
                }
                // Crossings are schematic, not authored paths; unfinished marks are
                // unresolved. Never choose transport art by parsing source labels.
                MapRecord::Hexside { .. } => continue,
            };
            let target = if ground {
                &mut art.terrain
            } else {
                &mut art.points
            };
            target.entry(hex).or_default().extend(names);
        }
        art
    }

    pub fn render_ground(&self, viewport: Viewport) -> String {
        let [width, height] = viewport.css_size();
        let visible = |hex| {
            let c = viewport.to_screen(center(hex));
            let margin = 90.0 * viewport.zoom();
            c.x > -width * 0.25 - margin
                && c.x < width * 1.25 + margin
                && c.y > -height * 0.25 - margin
                && c.y < height * 1.25 + margin
        };
        let polygon = |hex, inset| {
            let c = center(hex);
            corners(hex)
                .iter()
                .map(|p| {
                    format!(
                        "{:.2},{:.2}",
                        c.x + (p.x - c.x) * inset,
                        c.y + (p.y - c.y) * inset
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        let mut html = String::from("<g class=terrain-art><defs>");
        for (name, asset) in manifest()["assets"].as_object().unwrap() {
            if asset["kind"] == "surface" {
                write!(html, r#"<pattern id="art-{name}" width="1" height="1" patternContentUnits="objectBoundingBox"><image href="/art/terrain-v1/{}" width="1" height="1" preserveAspectRatio="xMidYMid slice"/></pattern>"#, asset["path"].as_str().unwrap()).unwrap();
            }
        }
        html.push_str("</defs>");
        // Paint all neutral cells first: overlapping section identities remain separate,
        // and an unknown neighboring section must not erase observed artwork.
        // Batch only the paint, not identities or hit testing, into one compound path.
        html.push_str(r##"<path class="terrain-art-unknown" fill="#d8dcdf" d=""##);
        for &hex in hexes().iter().filter(|&&hex| visible(hex)) {
            write!(html, "M{}Z", polygon(hex, 1.0).replace(' ', "L")).unwrap();
        }
        html.push_str("\"/>");
        for (&hex, materials) in self.terrain.iter().filter(|(hex, _)| visible(**hex)) {
            if materials.len() == 1 {
                let material = materials.first().unwrap();
                write!(html, r#"<polygon class="terrain-art-cell terrain-art-surface" data-hex="{hex}" data-terrain="{material}" points="{}" fill="url(#art-{material})"/>"#, polygon(hex, 0.84)).unwrap();
                continue;
            }
            let bounds = corners(hex);
            let left = bounds.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let right = bounds.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let top = bounds.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let bottom = bounds.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            let id = hex.to_string().replace(' ', "-");
            write!(html, r#"<clipPath id="art-cell-{id}"><polygon points="{}"/></clipPath><g class="terrain-art-cell" data-hex="{hex}" clip-path="url(#art-cell-{id})">"#, polygon(hex, 0.84)).unwrap();
            // Equal swatches express coexistence, not surveyed area fractions or placement.
            let swatch = (right - left) / materials.len() as f64;
            for (index, material) in materials.iter().enumerate() {
                write!(html, r#"<rect class="terrain-art-surface" data-terrain="{material}" x="{:.2}" y="{top:.2}" width="{swatch:.2}" height="{:.2}" fill="url(#art-{material})"/>"#, left + index as f64 * swatch, bottom - top).unwrap();
            }
            html.push_str("</g>");
        }
        html.push_str("</g>");
        html
    }

    pub fn render_points(&self, viewport: Viewport) -> String {
        let [width, height] = viewport.css_size();
        let visible = |hex| {
            let c = viewport.to_screen(center(hex));
            let margin = 90.0 * viewport.zoom();
            c.x > -margin && c.x < width + margin && c.y > -margin && c.y < height + margin
        };
        let mut locations = String::new();
        if viewport.zoom() >= 0.22 {
            for (&hex, points) in self.points.iter().filter(|(hex, _)| visible(**hex)) {
                let c = viewport.to_screen(center(hex));
                let size = 38.0 * viewport.zoom();
                for (index, name) in points.iter().enumerate() {
                    let asset = &manifest()["assets"][name];
                    let x = c.x + (index as f64 - (points.len() - 1) as f64 / 2.0) * size * 0.7;
                    write!(locations, r#"<image class="terrain-art-point" data-hex="{hex}" data-feature="{name}" href="/art/terrain-v1/{}" x="{:.2}" y="{:.2}" width="{size:.2}" height="{size:.2}" preserveAspectRatio="xMidYMid meet"/>"#, asset["path"].as_str().unwrap(), x - size / 2.0, c.y - size / 2.0).unwrap();
                }
            }
        }
        locations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn artwork_preserves_partial_terrain_and_does_not_classify_unresolved_marks() {
        let provenance = json!({"source_refs":["synthetic"],"reviewer":"test","date":"2026-10-25"});
        let records = [
            json!({"kind":"terrain","hex":"C3617","record":{"observation":{"status":"provisional","value":"rough"},"provenance":provenance}}),
            json!({"kind":"terrain","hex":"C3617","record":{"observation":{"status":"provisional","value":"salt_marsh"},"provenance":provenance}}),
            json!({"kind":"terrain","hex":"C3618","record":{"observation":{"status":"unresolved","source_label":"Clear?"},"provenance":provenance}}),
            json!({"kind":"point","hex":"C3617","record":{"observation":{"status":"reviewed","value":{"type":"well"}},"provenance":provenance}}),
            json!({"kind":"point","hex":"C3618","record":{"observation":{"status":"unresolved","source_label":"Unfinished railroad?"},"provenance":provenance}}),
        ].into_iter().map(|value| serde_json::from_value(value).unwrap()).collect();
        let art = Artwork::new(&[records]);
        let viewport =
            Viewport::new(center("C3617".parse().unwrap()), 1.0, [500.0, 400.0], 1.0).unwrap();
        let ground = art.render_ground(viewport);
        let points = art.render_points(viewport);
        assert!(ground.contains("data-terrain=\"rough\""));
        assert!(ground.contains("data-terrain=\"salt_marsh\""));
        assert!(!ground.contains("data-terrain=\"clear\""));
        assert!(ground.contains("class=\"terrain-art-unknown\" fill=\"#d8dcdf\""));
        assert!(!ground.contains("data-hex=\"C3618\""));
        assert!(points.contains("overlays/well.png"));
        assert!(!points.contains("railroad"));
    }
}
