//! Reference identities, counter versions and assets, never live game state.
//! The catalog/inventory modules read the existing JSON contracts unchanged;
//! these layered types are for reviewed catalog records, not automatic promotion
//! of the current provisional candidates into historical units.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod catalog;
pub mod census;
pub mod inventory;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanonicalUnitId(pub String);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalUnit {
    pub id: CanonicalUnitId,
    pub side: String,
    pub nationality: String,
    pub kind: String,
    pub names: Vec<String>,
    pub aliases: Vec<String>,
    pub source_refs: Vec<String>,
    pub status: IdentityStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityStatus {
    Provisional,
    SourceIdentified,
    AliasResolved,
    Verified,
    Unresolved,
    Fixture,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterRepresentation {
    /// None while the source slot's identity is still unresolved.
    pub unit_id: Option<CanonicalUnitId>,
    pub gpid: Option<String>,
    pub sheet: Option<String>,
    pub printed_label: String,
    pub images: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateVersion {
    /// A counter/symbol axis, not a separate unit or sprite identity.
    pub axis: String,
    pub level: String,
    pub source: String,
}

/// An opaque simulation-owned reference. No live strength or other stats.
///
/// ```compile_fail
/// use cna_catalog::ScenarioInstanceRef;
/// let instance = ScenarioInstanceRef { id: "instance-1".into(), stats: 42 };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInstanceRef {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRef {
    pub portrait: Option<String>,
    pub atlas: Option<String>,
    pub status: AssetStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    Missing,
    Candidate,
    Ready,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceValue<T> {
    pub value: Option<T>,
    pub status: ValueStatus,
    pub source_refs: Vec<String>,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueStatus {
    Sourced,
    Unverified,
    Provisional,
    Unresolved,
    NotApplicable,
    Conflict,
}

/// Reference establishment at a cited date/organization, not current strength.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Toe {
    pub unit_id: CanonicalUnitId,
    pub context: String,
    pub personnel: ReferenceValue<u32>,
    pub equipment: BTreeMap<String, ReferenceValue<u32>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Equipment {
    pub id: String,
    pub name: String,
    /// Keys identify cited characteristics, not simulation-computed values.
    pub characteristics: BTreeMap<String, ReferenceValue<f64>>,
}
