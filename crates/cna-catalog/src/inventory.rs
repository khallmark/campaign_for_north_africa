//! Existing identity inventory, separate from simulation roles and game state.
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Inventory {
    schema_version: u32,
    pub entries: Vec<Identity>,
}

#[derive(Deserialize)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub role_id: Option<String>,
    pub side: String,
    pub entity_kind: String,
    pub identity_status: String,
    pub source_entry_ids: Vec<String>,
    pub sprite: Artwork,
}

#[derive(Deserialize)]
pub struct Artwork {
    pub portrait_path: Option<String>,
    pub atlas_path: Option<String>,
    pub status: String,
}

impl Artwork {
    pub fn ready(&self) -> bool {
        self.status == "ready" && self.portrait_path.is_some() && self.atlas_path.is_some()
    }
}

impl Inventory {
    pub fn parse(json: &str) -> Result<Self, String> {
        let inventory: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if inventory.schema_version != 1 {
            return Err("Unsupported identity inventory schema".into());
        }
        let mut ids = std::collections::HashSet::new();
        if inventory.entries.iter().any(|entry| !ids.insert(&entry.id)) {
            return Err("Duplicate canonical identity ID".into());
        }
        Ok(inventory)
    }

    pub fn find(&self, id: &str) -> Option<&Identity> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn source_art(&self, source_id: &str) -> Option<(Option<&str>, &'static str)> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.source_entry_ids.iter().any(|id| id == source_id))?;
        Some(if entry.sprite.ready() {
            (
                entry.sprite.portrait_path.as_deref(),
                "Unique identity artwork",
            )
        } else {
            (None, "Unique art missing — shared role placeholder")
        })
    }
}
