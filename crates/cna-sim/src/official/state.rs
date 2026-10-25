//! State representation, not a scenario loader or a certification of rule legality.
use crate::contract::route::Points;
use cna_catalog::{CanonicalUnitId, StateVersion};
use cna_map::hex::MapLocation;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
};

// Separate dimensions prevent passing fuel as water. Fractions remain exact;
// individual rules decide admissible quantities and rounding, never the serializer.
macro_rules! quantities {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Points);

        impl $name {
            pub fn new(numerator: u32, denominator: u32) -> Result<Self, &'static str> {
                Ok(Self(Points {
                    numerator,
                    denominator: NonZeroU32::new(denominator).ok_or("zero quantity denominator")?,
                }))
            }

            pub fn whole(points: u32) -> Self {
                Self(Points { numerator: points, denominator: NonZeroU32::MIN })
            }
        }

        impl Default for $name {
            fn default() -> Self { Self::whole(0) }
        }
    )+};
}
quantities!(
    ToePoints,
    FuelPoints,
    AmmunitionPoints,
    StoresPoints,
    WaterPoints,
    TruckPoints
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Axis,
    Commonwealth,
}

/// Stored choice only. Active kernels and mode compatibility are CNA-279.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Detailed,
    DetailedLogisticsAbstractAir58,
    Abstract32,
    Abstract47,
}

/// Campaign track coordinates, not Gregorian days: L21 §5.1 and the printed
/// 111-week turn track (September III 1940 through January I 1943).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "CalendarWire")]
pub struct Calendar {
    game_turn: u16,
    operations_stage: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CalendarWire {
    game_turn: u16,
    operations_stage: u8,
}

impl TryFrom<CalendarWire> for Calendar {
    type Error = &'static str;
    fn try_from(value: CalendarWire) -> Result<Self, Self::Error> {
        Self::new(value.game_turn, value.operations_stage)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampaignDate {
    pub year: u16,
    pub month: u8,
    pub week: u8,
}

impl Calendar {
    pub fn new(game_turn: u16, operations_stage: u8) -> Result<Self, &'static str> {
        if !(1..=111).contains(&game_turn) || !(1..=3).contains(&operations_stage) {
            return Err("calendar requires Game-Turn 1..111 and Operations Stage 1..3");
        }
        Ok(Self {
            game_turn,
            operations_stage,
        })
    }

    pub fn game_turn(self) -> u16 {
        self.game_turn
    }
    pub fn operations_stage(self) -> u8 {
        self.operations_stage
    }

    pub fn date(self) -> CampaignDate {
        let week = self.game_turn + 1;
        let month = 8 + week / 4;
        CampaignDate {
            year: 1940 + month / 12,
            month: (month % 12 + 1) as u8,
            week: (week % 4 + 1) as u8,
        }
    }

    /// Calendar arithmetic only. The phase engine must authorize advancing;
    /// this does not execute phases, refresh units, or consume supplies.
    pub fn next_operations_stage(self) -> Option<Self> {
        if self.operations_stage < 3 {
            Self::new(self.game_turn, self.operations_stage + 1).ok()
        } else {
            Self::new(self.game_turn + 1, 1).ok()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supplies {
    pub fuel: FuelPoints,
    pub ammunition: AmmunitionPoints,
    pub stores: StoresPoints,
    pub water: WaterPoints,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TruckType {
    Light,
    Medium,
    Heavy,
}

/// AL21 §53.11–53.13. Lines are assignments, not additional vehicles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TruckLine {
    First,
    Second,
    Third,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitInstance {
    pub unit_id: CanonicalUnitId,
    pub side: Side,
    pub location: MapLocation,
    pub counter_states: BTreeMap<String, StateVersion>,
    /// Weapon-system/reference equipment ID -> current TOE points, not vehicles.
    pub toe: BTreeMap<String, ToePoints>,
    pub supplies: Supplies,
    pub trucks: BTreeMap<TruckType, BTreeMap<TruckLine, TruckPoints>>,
}

/// Permanent assignment and temporary attachment are kept separately (§19).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Organization {
    pub assigned_to: Option<String>,
    pub attached_to: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "reference",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CargoHolder {
    Instance(String),
    Location(MapLocation),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoBatch {
    pub side: Side,
    pub holder: CargoHolder,
    pub supplies: Supplies,
}

/// References reserved by a pending action, without embedding or copying units.
/// Action-specific legality and resource reservation accounting belong to reducers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commitment {
    pub side: Side,
    pub instances: BTreeSet<String>,
    pub cargo_batches: BTreeSet<String>,
}

/// Serializable ChaCha8 stream-0 cursor per the command contract. Recorded
/// draws are supplied by `official::rng`, not by this representation itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RngState {
    pub seed: [u8; 32],
    pub word_position: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceDigests {
    pub ruleset_id: String,
    pub ruleset: String,
    pub map: String,
    pub catalog: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameState {
    pub revision: u64,
    pub calendar: Calendar,
    pub mode: Mode,
    pub instances: BTreeMap<String, UnitInstance>,
    /// Key is the member instance; values point at its assigned/attached parent.
    pub organizations: BTreeMap<String, Organization>,
    pub cargo_batches: BTreeMap<String, CargoBatch>,
    pub pending_commitments: BTreeMap<String, Commitment>,
    pub rng: RngState,
    pub references: ReferenceDigests,
}

impl GameState {
    /// Check references before installing a deserialized state. Catalog membership
    /// is not historical/stat verification, and digest syntax is not a content
    /// match: loaders must also compare the selected reference bundle (CNA-270).
    /// Errors are internal diagnostics, NEVER viewer-facing command rejections.
    pub fn validate(&self, catalog_ids: &BTreeSet<CanonicalUnitId>) -> Result<(), String> {
        let require_instance = |id: &str| {
            if self.instances.contains_key(id) {
                Ok(())
            } else {
                Err(format!("unknown instance reference: {id}"))
            }
        };
        for id in self
            .instances
            .keys()
            .chain(self.cargo_batches.keys())
            .chain(self.pending_commitments.keys())
        {
            if id.trim().is_empty() {
                return Err("empty state identifier".into());
            }
        }
        if self.references.ruleset_id.trim().is_empty() {
            return Err("empty ruleset identifier".into());
        }
        for digest in [
            &self.references.ruleset,
            &self.references.map,
            &self.references.catalog,
        ] {
            if !digest.strip_prefix("sha256:").is_some_and(|hex| {
                hex.len() == 64
                    && hex
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            }) {
                return Err("invalid reference digest".into());
            }
        }
        for instance in self.instances.values() {
            if instance.unit_id.0.trim().is_empty() || !catalog_ids.contains(&instance.unit_id) {
                return Err(format!("unknown catalog unit: {}", instance.unit_id.0));
            }
            for (axis, version) in &instance.counter_states {
                if axis.trim().is_empty()
                    || axis != &version.axis
                    || version.level.trim().is_empty()
                    || version.source.trim().is_empty()
                {
                    return Err("invalid counter state reference".into());
                }
            }
            if instance.toe.keys().any(|id| id.trim().is_empty()) {
                return Err("empty weapon-system reference".into());
            }
        }
        for (member, organization) in &self.organizations {
            require_instance(member)?;
            for parent in organization
                .assigned_to
                .iter()
                .chain(&organization.attached_to)
            {
                require_instance(parent)?;
            }
        }
        for batch in self.cargo_batches.values() {
            if let CargoHolder::Instance(id) = &batch.holder {
                require_instance(id)?;
            }
        }
        for commitment in self.pending_commitments.values() {
            for id in &commitment.instances {
                require_instance(id)?;
            }
            for id in &commitment.cargo_batches {
                if !self.cargo_batches.contains_key(id) {
                    return Err(format!("unknown cargo batch reference: {id}"));
                }
            }
        }
        Ok(())
    }
}
