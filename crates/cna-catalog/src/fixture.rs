//! Source-labelled consumer samples, never a certified roster or live scenario.

use crate::{CanonicalUnit, CanonicalUnitId, IdentityStatus, Toe};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub contract_version: u32,
    pub status: IdentityStatus,
    pub note: String,
    pub units: Vec<FixtureUnit>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureUnit {
    pub unit: CanonicalUnit,
    /// Sample-only association from the cited setup, not a live attachment order.
    pub attached_to: Option<CanonicalUnitId>,
    pub toe: Toe,
}

/// Load the bundled SC21 60.31/60.41 sample. All identities are fixtures;
/// establishment values remain unverified and must not certify a scenario.
pub fn graziani() -> Result<Fixture, serde_json::Error> {
    serde_json::from_str(include_str!("../fixtures/graziani-sample.json"))
}
