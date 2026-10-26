//! Auditable transcription batches expanded into the existing map-v1 vocabulary.
//! Connectivity is section-local evidence, never an official movement graph.
use crate::{
    geometry::{contains_hex, hexes},
    hex::{HexId, HexsideId},
    terrain::{
        HexsideFeature, MapRecord, Observation, PointFeature, Provenance, Record, Terrain,
        TransportDepiction,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Survey {
    pub id: String,
    pub label: String,
    pub notes: String,
    pub provenance: Provenance,
    pub coverage: Coverage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Coverage {
    Terrain {
        hexes: Vec<HexId>,
        observation: Observation<Terrain>,
    },
    Point {
        hexes: Vec<HexId>,
        observation: Observation<PointFeature>,
    },
    Hexside {
        sides: Vec<HexsideId>,
        observation: Observation<HexsideFeature>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        depiction: Option<TransportDepiction>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub version: u32,
    pub sources: BTreeMap<String, String>,
    pub surveys: Vec<Survey>,
    #[serde(default)]
    pub examinations: BTreeMap<HexId, Examination>,
}

/// Detector coverage is independent of terrain completeness and human review.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Examination {
    pub crop: [i32; 4],
    pub surface: String,
    pub families: BTreeMap<String, String>,
}

impl Survey {
    pub fn records(&self) -> Vec<MapRecord> {
        let record = |observation, provenance| Record {
            observation,
            provenance,
            adjudication_id: None,
        };
        match &self.coverage {
            Coverage::Terrain { hexes, observation } => hexes
                .iter()
                .map(|&hex| MapRecord::Terrain {
                    hex,
                    record: record(observation.clone(), self.provenance.clone()),
                })
                .collect(),
            Coverage::Point { hexes, observation } => hexes
                .iter()
                .map(|&hex| MapRecord::Point {
                    hex,
                    record: Record {
                        observation: observation.clone(),
                        provenance: self.provenance.clone(),
                        adjudication_id: None,
                    },
                })
                .collect(),
            Coverage::Hexside {
                sides,
                observation,
                depiction,
            } => sides
                .iter()
                .map(|&side| MapRecord::Hexside {
                    side,
                    depiction: *depiction,
                    record: Record {
                        observation: observation.clone(),
                        provenance: self.provenance.clone(),
                        adjudication_id: None,
                    },
                })
                .collect(),
        }
    }
}

impl Model {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.sources.is_empty() {
            return Err("expected model version 1 and pinned sources".into());
        }
        for hash in self.sources.values() {
            if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("expected source SHA256".into());
            }
        }
        if !self.examinations.is_empty() && self.examinations.len() != hexes().len() {
            return Err("examination ledger must cover the entire geometry inventory".into());
        }
        for (hex, examination) in &self.examinations {
            if !contains_hex(*hex)
                || examination.crop[2] <= 0
                || examination.crop[3] <= 0
                || !["land", "water", "mixed_water_land", "unresolved"]
                    .contains(&examination.surface.as_str())
                || examination.families.len() != 5
                || ["surface", "terrain", "transport", "hexside", "locations"]
                    .iter()
                    .any(|family| !examination.families.contains_key(*family))
                || examination.families.values().any(|status| {
                    !["detected", "candidate", "not_detected", "unresolved"]
                        .contains(&status.as_str())
                })
            {
                return Err(format!("invalid automated examination for {hex}"));
            }
        }
        let mut ids = BTreeSet::new();
        for survey in &self.surveys {
            if matches!(&survey.coverage, Coverage::Hexside { depiction: Some(_), observation, .. }
                if !matches!(observation, Observation::Unresolved { .. }))
            {
                return Err(format!(
                    "construction depiction requires unresolved observation in {}",
                    survey.id
                ));
            }
            if survey.id.trim().is_empty()
                || !ids.insert(&survey.id)
                || survey.label.trim().is_empty()
                || survey.notes.trim().is_empty()
                || survey.provenance.reviewer.trim().is_empty()
                || survey.provenance.date.len() != 10
                || survey.provenance.source_refs.is_empty()
                || survey.records().is_empty()
            {
                return Err(format!("incomplete or duplicate survey {}", survey.id));
            }
            for reference in &survey.provenance.source_refs {
                if !self
                    .sources
                    .contains_key(reference.split('#').next().unwrap())
                {
                    return Err(format!("unpinned source {reference}"));
                }
            }
            let mut locations = BTreeSet::new();
            for record in survey.records() {
                let (hex, other, key, observation) = match record {
                    MapRecord::Terrain { hex, record } => (
                        hex,
                        None,
                        hex.to_string(),
                        serde_json::to_value(record.observation).unwrap(),
                    ),
                    MapRecord::Point { hex, record } => (
                        hex,
                        None,
                        hex.to_string(),
                        serde_json::to_value(record.observation).unwrap(),
                    ),
                    MapRecord::Hexside { side, record, .. } => (
                        side.hex(),
                        side.hex().neighbor(side.direction()),
                        format!("{}:{:?}", side.hex(), side.direction()),
                        serde_json::to_value(record.observation).unwrap(),
                    ),
                };
                if !contains_hex(hex)
                    || other.is_some_and(|h| !contains_hex(h))
                    || !locations.insert(key)
                {
                    return Err(format!(
                        "duplicate or out-of-section location in {}",
                        survey.id
                    ));
                }
                if observation["status"] == "fixture"
                    || observation["source_label"]
                        .as_str()
                        .is_some_and(|s| s.trim().is_empty())
                {
                    return Err(format!("invalid production observation in {}", survey.id));
                }
            }
        }
        Ok(())
    }
}

pub fn model() -> &'static Model {
    static MODEL: OnceLock<Model> = OnceLock::new();
    MODEL.get_or_init(|| {
        let mut model: Model = serde_json::from_str(include_str!("../../../data/map/surveys.json"))
            .expect("valid map surveys");
        let extracted: Model =
            serde_json::from_str(include_str!("../../../data/map/extracted-surveys.json"))
                .expect("valid extracted map surveys");
        assert_eq!(model.sources, extracted.sources, "extraction source pins");
        model.surveys.extend(extracted.surveys);
        model.examinations = extracted.examinations;
        model.validate().expect("source-backed map surveys");
        model
    })
}
