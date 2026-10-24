//! Existing identity inventory, separate from simulation roles and game state.
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct Inventory {
    schema_version: u32,
    pub entries: Vec<Identity>,
    #[serde(default, deserialize_with = "unique_redirects")]
    identity_redirects: BTreeMap<String, String>,
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
        for retired in inventory.identity_redirects.keys() {
            if ids.contains(retired) {
                return Err(format!("Identity {retired} is both live and redirected"));
            }
            if inventory.find(retired).is_none() {
                return Err(format!("Redirect {retired} has a cycle or missing target"));
            }
        }
        Ok(inventory)
    }

    pub fn find(&self, id: &str) -> Option<&Identity> {
        let mut resolved = id;
        // At most one edge per retired ID; exhausting this bound means a loop.
        // Keep lookup bounded even if a caller deserialized without parse().
        for _ in 0..=self.identity_redirects.len() {
            if let Some(next) = self.identity_redirects.get(resolved) {
                resolved = next;
            } else {
                return self.entries.iter().find(|entry| entry.id == resolved);
            }
        }
        None
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

// BTreeMap's default deserializer silently keeps the last value for duplicate
// JSON keys. A retired ID with two targets must fail, not depend on input order.
fn unique_redirects<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct RedirectVisitor;
    impl<'de> serde::de::Visitor<'de> for RedirectVisitor {
        type Value = BTreeMap<String, String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a redirect map with unique retired IDs")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut redirects = BTreeMap::new();
            while let Some((retired, target)) = map.next_entry::<String, String>()? {
                if redirects.insert(retired.clone(), target).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "Duplicate redirect: {retired}"
                    )));
                }
            }
            Ok(redirects)
        }
    }
    deserializer.deserialize_map(RedirectVisitor)
}
