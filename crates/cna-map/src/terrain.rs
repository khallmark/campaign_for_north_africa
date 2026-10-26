//! Source-labelled geography records, not movement costs or historical assignments.

use crate::hex::{HexId, HexsideId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    Clear,
    Gravel,
    SaltMarsh,
    HeavyVegetation,
    Rough,
    Mountain,
    Delta,
    Desert,
    MajorCity,
    Swamp,
    VillageBirOasis,
    FortificationLevelOne,
    FortificationLevelTwo,
    FortificationLevelThree,
    FriendlyMinefield,
    EnemyMinefield,
}

impl Terrain {
    pub const ALL: [Self; 16] = [
        Self::Clear,
        Self::Gravel,
        Self::SaltMarsh,
        Self::HeavyVegetation,
        Self::Rough,
        Self::Mountain,
        Self::Delta,
        Self::Desert,
        Self::MajorCity,
        Self::Swamp,
        Self::VillageBirOasis,
        Self::FortificationLevelOne,
        Self::FortificationLevelTwo,
        Self::FortificationLevelThree,
        Self::FriendlyMinefield,
        Self::EnemyMinefield,
    ];

    /// Exact printed row label, excluding superscript footnote markers.
    pub const fn row_name(self) -> &'static str {
        match self {
            Self::Clear => "Clear",
            Self::Gravel => "Gravel",
            Self::SaltMarsh => "Salt Marsh",
            Self::HeavyVegetation => "Heavy Vegetation",
            Self::Rough => "Rough",
            Self::Mountain => "Mountain",
            Self::Delta => "Delta",
            Self::Desert => "Desert",
            Self::MajorCity => "Major City",
            Self::Swamp => "Swamp",
            Self::VillageBirOasis => "Village/Bir/Oasis",
            Self::FortificationLevelOne => "Level One",
            Self::FortificationLevelTwo => "Level Two",
            Self::FortificationLevelThree => "Level Three",
            Self::FriendlyMinefield => "Friendly Minefield",
            Self::EnemyMinefield => "Enemy Minefield",
        }
    }

    pub fn source_ref(self) -> String {
        format!("docs/CNAv2.1.0/images/TEC.png#8.37/{}", self.row_name())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HexsideFeature {
    Railroad,
    Road,
    Track,
    Ridge,
    UpSlope,
    DownSlope,
    UpEscarpment,
    DownEscarpment,
    Wadi,
    MajorRiver,
    MinorRiver,
    Coast,
    Sea,
    Pipeline,
}

impl HexsideFeature {
    pub const ALL: [Self; 14] = [
        Self::Railroad,
        Self::Road,
        Self::Track,
        Self::Ridge,
        Self::UpSlope,
        Self::DownSlope,
        Self::UpEscarpment,
        Self::DownEscarpment,
        Self::Wadi,
        Self::MajorRiver,
        Self::MinorRiver,
        Self::Coast,
        Self::Sea,
        Self::Pipeline,
    ];

    pub const fn row_name(self) -> &'static str {
        match self {
            Self::Railroad => "Railroad",
            Self::Road => "Road",
            Self::Track => "Track",
            Self::Ridge => "Ridge",
            Self::UpSlope => "Up Slope",
            Self::DownSlope => "Down Slope",
            Self::UpEscarpment => "Up Escarpment",
            Self::DownEscarpment => "Down Escarpment",
            Self::Wadi => "Wadi",
            Self::MajorRiver => "Major River",
            Self::MinorRiver => "Minor River",
            Self::Coast => "Coast",
            Self::Sea => "Sea",
            Self::Pipeline => "Pipeline",
        }
    }

    pub fn source_ref(self) -> String {
        match self {
            Self::Coast => "docs/CNAv2.1.0/Rules_Air&Logistics_07-2021.pdf#36.3".into(),
            Self::Sea => "docs/CNAv2.1.0/Rules_Land_07-2021.pdf#5/Commonwealth Fleet Phase".into(),
            Self::Pipeline => "docs/CNAv2.1.0/Rules_Air&Logistics_07-2021.pdf#52.2".into(),
            _ => format!("docs/CNAv2.1.0/images/TEC.png#8.37/{}", self.row_name()),
        }
    }

    /// Directional features are stored looking from the canonical side's hex to its neighbour.
    pub const fn reversed(self) -> Self {
        match self {
            Self::UpSlope => Self::DownSlope,
            Self::DownSlope => Self::UpSlope,
            Self::UpEscarpment => Self::DownEscarpment,
            Self::DownEscarpment => Self::UpEscarpment,
            other => other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FortificationLevel {
    One,
    Two,
    Three,
}

/// A row reference, not a copied capacity or an inferred port at a particular hex.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortReference {
    Tripoli,
    Bizerta,
    Alexandria,
    Tobruk,
    Benghazi,
    MersaMatruh,
    Bardia,
    Sollum,
    Derna,
    AllOthers,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PointFeature {
    MajorCity,
    Village,
    Bir,
    Oasis,
    Well,
    Airfield,
    LandingStrip,
    FlyingBoatBasin,
    Fortification { level: FortificationLevel },
    Port { efficiency_ref: PortReference },
}

/// An unresolved source label has no typed value. In particular it cannot silently
/// deserialize as Clear, or make an unsupported "Minor City" into reviewed terrain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation<T> {
    Reviewed { value: T },
    Provisional { value: T },
    Fixture { value: T },
    Unresolved { source_label: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub source_refs: Vec<String>,
    pub reviewer: String,
    pub date: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record<T> {
    pub observation: Observation<T>,
    pub provenance: Provenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjudication_id: Option<String>,
}

/// What the source draws, not construction completion or operational connectivity.
/// Production hints accompany unresolved observations, never typed movement links.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportDepiction {
    UnfinishedRoad,
    UnfinishedRailroad,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MapRecord {
    Terrain {
        hex: HexId,
        record: Record<Terrain>,
    },
    Hexside {
        side: HexsideId,
        record: Record<HexsideFeature>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        depiction: Option<TransportDepiction>,
    },
    Point {
        hex: HexId,
        record: Record<PointFeature>,
    },
}
