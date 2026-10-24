use cna_map::fixture::corridor;
use cna_map::geometry::{center, corners, hex_at};
use cna_map::hex::{Direction, HexId, MapSection};
use cna_map::terrain::{HexsideFeature, MapRecord, Observation, Record};
use cna_map::MAP_CONTRACT_VERSION;
use std::collections::BTreeSet;

fn fixture_value<T>(record: &Record<T>) -> &T {
    assert!(!record.provenance.source_refs.is_empty());
    assert!(!record.provenance.reviewer.is_empty());
    assert!(!record.provenance.date.is_empty());
    match &record.observation {
        Observation::Fixture { value } => value,
        _ => panic!("consumer fixture must never contain certified or ambiguous statuses"),
    }
}

#[test]
fn fixture_every_record_is_explicitly_synthetic_and_has_required_features() {
    assert_eq!(MAP_CONTRACT_VERSION, 1);
    let records = corridor();
    let mut features = Vec::new();
    for record in &records {
        match record {
            MapRecord::Terrain { record, .. } => {
                fixture_value(record);
            }
            MapRecord::Hexside { record, .. } => features.push(*fixture_value(record)),
            MapRecord::Point { record, .. } => {
                fixture_value(record);
            }
        }
    }
    for required in [
        HexsideFeature::UpEscarpment,
        HexsideFeature::Road,
        HexsideFeature::Track,
        HexsideFeature::Coast,
    ] {
        assert!(features.contains(&required), "missing {required:?}");
    }
}

#[test]
fn fixture_corridor_is_connected_with_source_geometry_and_six_board_neighbours() {
    let records = corridor();
    let terrain: Vec<HexId> = records
        .iter()
        .filter_map(|r| match r {
            MapRecord::Terrain { hex, .. } => Some(*hex),
            _ => None,
        })
        .collect();
    let ids: BTreeSet<_> = terrain.iter().copied().collect();
    assert_eq!(ids.len(), 44);
    assert_eq!(terrain.len(), ids.len(), "duplicate terrain records");
    for id in ["C3926", "C4321"] {
        assert!(ids.contains(&id.parse().unwrap()));
    }
    for &hex in &ids {
        assert_eq!(hex.section(), MapSection::C);
        assert!((30..=43).contains(&hex.col()));
        assert_eq!(hex_at(center(hex)), Some(hex));
        assert!(corners(hex)
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite()));
        // This sample is wholly inside the census polygon: no board edges needed.
        // Outside-sample neighbours are real lattice neighbours, not fixture terrain.
        for dir in Direction::ALL {
            let neighbour = hex.neighbor(dir).unwrap();
            assert_eq!(hex_at(center(neighbour)), Some(neighbour), "{hex} {dir:?}");
        }
    }
    let mut reached = BTreeSet::from([terrain[0]]);
    loop {
        let before = reached.len();
        for hex in reached.clone() {
            for dir in Direction::ALL {
                if let Some(n) = hex.neighbor(dir).filter(|n| ids.contains(n)) {
                    reached.insert(n);
                }
            }
        }
        if reached.len() == before {
            break;
        }
    }
    assert_eq!(
        reached, ids,
        "corridor must not split into disconnected islands"
    );
    for record in records {
        if let MapRecord::Hexside { side, .. } = record {
            assert!(ids.contains(&side.hex()));
            assert!(ids.contains(&side.hex().neighbor(side.direction()).unwrap()));
        }
    }
    // Source numbers evaluated independently: C grid col_base=63, dx=72.95.
    for (id, x, y) in [("C3926", 7853.0, 1734.8), ("C4321", 7426.75, 1443.0)] {
        let point = center(id.parse().unwrap());
        assert!((point.x - x).abs() < 1e-9);
        assert!((point.y - y).abs() < 1e-9);
    }
}
