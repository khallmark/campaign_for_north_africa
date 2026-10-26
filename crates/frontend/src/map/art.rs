//! Original artwork depicts typed observations, never inferred geography.
use cna_map::{
    geometry::center,
    hex::HexId,
    terrain::{MapRecord, Observation},
    viewport::{ScreenPoint, Viewport},
};
use serde::Deserialize;
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
    points: BTreeMap<HexId, BTreeSet<String>>,
}

impl Artwork {
    pub fn new(records: &[Vec<MapRecord>]) -> Self {
        let mut art = Self {
            points: BTreeMap::new(),
        };
        for record in records.iter().flatten() {
            let (hex, names) = match record {
                MapRecord::Terrain { hex, record } => {
                    let Some(value) = known(&record.observation) else {
                        continue;
                    };
                    let key = serde_json::to_value(value).unwrap();
                    let recipe = &manifest()["recipes"]["terrain"][key.as_str().unwrap()];
                    if recipe["surface"].is_string() {
                        // Ground is baked once from the same typed observations.
                        continue;
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
                    (*hex, vec![name.to_owned()])
                }
                // Crossings are schematic, not authored paths; unfinished marks are
                // unresolved. Never choose transport art by parsing source labels.
                MapRecord::Hexside { .. } => continue,
            };
            art.points.entry(hex).or_default().extend(names);
        }
        art
    }

    pub fn render_ground(&self, viewport: Viewport) -> String {
        #[derive(Deserialize)]
        struct Level {
            z: u32,
            width: u32,
            height: u32,
            columns: u32,
            rows: u32,
        }
        #[derive(Deserialize)]
        struct Ground {
            board_width: u32,
            board_height: u32,
            levels: Vec<Level>,
        }
        static GROUND: OnceLock<Ground> = OnceLock::new();
        let ground = GROUND.get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../public/art/terrain-v1/ground/manifest.json"
            ))
            .expect("verified original ground tiles")
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
        let range = |level: &Level| {
            let sx = f64::from(level.width) / f64::from(ground.board_width) / 512.0;
            let sy = f64::from(level.height) / f64::from(ground.board_height) / 512.0;
            [
                (low.x * sx).floor().max(0.0) as u32,
                ((high.x * sx).floor().max(0.0) as u32).min(level.columns - 1),
                (low.y * sy).floor().max(0.0) as u32,
                ((high.y * sy).floor().max(0.0) as u32).min(level.rows - 1),
            ]
        };
        let desired = viewport.zoom() * viewport.device_pixel_ratio();
        let mut index = ground
            .levels
            .iter()
            .position(|level| {
                f64::from(level.width) / f64::from(ground.board_width) >= desired
                    && f64::from(level.height) / f64::from(ground.board_height) >= desired
            })
            .unwrap_or(ground.levels.len() - 1);
        while index > 0 {
            let [x0, x1, y0, y1] = range(&ground.levels[index]);
            if (x1 - x0 + 1) * (y1 - y0 + 1) <= 100 {
                break;
            }
            index -= 1;
        }
        let level = &ground.levels[index];
        let [x0, x1, y0, y1] = range(level);
        let sx = f64::from(ground.board_width) / f64::from(level.width);
        let sy = f64::from(ground.board_height) / f64::from(level.height);
        let mut html = format!(
            r#"<g class="terrain-art"><image href="/art/terrain-v1/ground/0/0_0.webp" width="{}" height="{}" preserveAspectRatio="none"/>"#,
            ground.board_width, ground.board_height
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                write!(html, r#"<image class="terrain-art-tile" href="/art/terrain-v1/ground/{}/{x}_{y}.webp" x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="none"/>"#,
                    level.z, f64::from(x * 512) * sx, f64::from(y * 512) * sy,
                    f64::from((level.width - x * 512).min(512)) * sx,
                    f64::from((level.height - y * 512).min(512)) * sy).unwrap();
            }
        }
        html.push_str("</g>");
        html
    }

    pub fn render_points(&self, viewport: Viewport) -> String {
        let [width, height] = viewport.css_size();
        let visible = |hex| {
            let c = viewport.to_screen(center(hex));
            let margin = 90.0 * viewport.zoom();
            c.x > -width * 0.25 - margin
                && c.x < width * 1.25 + margin
                && c.y > -height * 0.25 - margin
                && c.y < height * 1.25 + margin
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
    fn point_artwork_does_not_classify_unresolved_marks() {
        let provenance = json!({"source_refs":["synthetic"],"reviewer":"test","date":"2026-10-25"});
        let records = [
            json!({"kind":"point","hex":"C3617","record":{"observation":{"status":"reviewed","value":{"type":"well"}},"provenance":provenance}}),
            json!({"kind":"point","hex":"C3618","record":{"observation":{"status":"unresolved","source_label":"Unfinished railroad?"},"provenance":provenance}}),
        ].into_iter().map(|value| serde_json::from_value(value).unwrap()).collect();
        let art = Artwork::new(&[records]);
        let viewport =
            Viewport::new(center("C3617".parse().unwrap()), 1.0, [500.0, 400.0], 1.0).unwrap();
        let points = art.render_points(viewport);
        assert!(points.contains("overlays/well.png"));
        assert!(!points.contains("railroad"));
    }
}
