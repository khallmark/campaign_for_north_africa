//! Existing rulebook JSON contract; reference records are not scenario units.
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct Catalog {
    schema_version: u32,
    pub coverage: Coverage,
    pub entries: Vec<Record>,
    pub profiles: Vec<Record>,
    pub stat_definitions: Vec<StatDefinition>,
    pub sources: Vec<Source>,
    unit_types: Vec<UnitType>,
    sprites: Vec<Sprite>,
}

#[derive(Deserialize)]
pub struct Coverage {
    pub complete_historical_roster: bool,
    pub summary: String,
    pub gaps: Vec<String>,
}

#[derive(Deserialize)]
pub struct Record {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub side: String,
    pub nationality: String,
    pub unit_type: String,
    pub profile_id: Option<String>,
    pub verification: String,
    pub source_refs: Vec<String>,
    pub stats: BTreeMap<String, Stat>,
    pub sprite: Option<Sprite>,
    pub formation_reference: Option<FormationReference>,
    #[serde(skip)]
    pub artwork_note: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Deserialize)]
pub struct FormationReference {
    pub schema_version: u32,
    pub base_profile_id: Option<String>,
    pub base_profile_name: Option<String>,
    pub base_profile_note: String,
    pub base_stats: BTreeMap<String, Stat>,
    pub printed_profile_code: Option<String>,
    pub catalog_profile_id: Option<String>,
    pub arrival: FormationArrival,
    pub components: Vec<FormationComponent>,
    pub arrival_totals: BTreeMap<String, Stat>,
    pub limitations: String,
}

#[derive(Deserialize)]
pub struct FormationArrival {
    pub toe_raw: Option<String>,
    pub arrives_raw: Option<String>,
    pub note: String,
    pub stats: BTreeMap<String, Stat>,
    pub source_refs: Vec<String>,
    pub related_record_id: Option<String>,
    pub toe_strength: Stat,
    pub morale: Stat,
}

#[derive(Deserialize)]
pub struct FormationComponent {
    pub record_id: Option<String>,
    pub name: String,
    pub label: String,
    pub kind: String,
    pub toe: Stat,
    pub per_toe_stats: BTreeMap<String, Stat>,
    pub land_barrage_toe: Option<u32>,
}

#[derive(Deserialize)]
pub struct Stat {
    pub value: Option<f64>,
    pub status: String,
    pub source_refs: Vec<String>,
    pub note: String,
    pub raw: Option<String>,
    #[serde(default)]
    pub conditional: bool,
    pub unit: Option<String>,
}

#[derive(Deserialize)]
pub struct StatDefinition {
    pub id: String,
    pub name: String,
    pub unit: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub path: String,
    pub section: Option<String>,
    pub pdf_page: Option<u32>,
    pub note: String,
}

#[derive(Clone, Deserialize)]
pub struct Sprite {
    #[serde(default)]
    pub id: String,
    pub path: String,
}

#[derive(Deserialize)]
struct UnitType {
    id: String,
    name: String,
    sprite_id: String,
}

impl Catalog {
    pub fn unit_type_name(&self, id: &str) -> Option<&str> {
        self.unit_types
            .iter()
            .find(|unit_type| unit_type.id == id)
            .map(|unit_type| unit_type.name.as_str())
    }

    pub fn parse(json: &str) -> Result<Self, String> {
        let mut catalog: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if catalog.schema_version != 1 {
            return Err("Unsupported catalog schema".into());
        }
        // Profiles are reference records, not formations or deployable scenario units.
        for profile in &mut catalog.profiles {
            profile.kind = "profile".into();
            if profile.sprite.is_none() {
                profile.sprite = catalog
                    .unit_types
                    .iter()
                    .find(|role| role.id == profile.unit_type)
                    .and_then(|role| {
                        catalog
                            .sprites
                            .iter()
                            .find(|sprite| sprite.id == role.sprite_id)
                    })
                    .cloned();
            }
        }
        Ok(catalog)
    }
}
