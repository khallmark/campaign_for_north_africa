//! Synthetic geography on source-derived corridor identities. Never certified terrain.

use crate::terrain::MapRecord;

/// Load fixture-only records. Obtain centers/corners/edges through `crate::geometry`
/// and CSS projection through `crate::viewport`, not a second geometry table.
/// Neighbours outside this small sample remain unknown, not impassable board edges.
pub fn corridor() -> Vec<MapRecord> {
    serde_json::from_str(include_str!("../fixtures/corridor-fixture.json"))
        .expect("checked-in corridor fixture must match map contract v1")
}
