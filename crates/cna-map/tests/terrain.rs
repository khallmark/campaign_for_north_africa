use cna_map::hex::{Direction, HexId, HexsideId};
use cna_map::terrain::{HexsideFeature, MapRecord, Observation, PointFeature, Terrain};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[test]
fn terrain_schema_covers_every_source_row_and_enum_with_a_reference() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../docs/map/map-v1.schema.json")).unwrap();
    let terrain_rows = schema["$defs"]["terrain"]["oneOf"].as_array().unwrap();
    let hexside_rows = schema["$defs"]["hexside"]["oneOf"].as_array().unwrap();
    let expected_terrain = [
        "Clear",
        "Gravel",
        "Salt Marsh",
        "Heavy Vegetation",
        "Rough",
        "Mountain",
        "Delta",
        "Desert",
        "Major City",
        "Swamp",
        "Village/Bir/Oasis",
        "Level One",
        "Level Two",
        "Level Three",
        "Friendly Minefield",
        "Enemy Minefield",
    ];
    assert_eq!(
        terrain_rows
            .iter()
            .map(|r| r["title"].as_str().unwrap())
            .collect::<Vec<_>>(),
        expected_terrain
    );
    let expected_hexside = [
        "Railroad",
        "Road",
        "Track",
        "Ridge",
        "Up Slope",
        "Down Slope",
        "Up Escarpment",
        "Down Escarpment",
        "Wadi",
        "Major River",
        "Minor River",
    ];
    assert_eq!(
        hexside_rows
            .iter()
            .filter(|r| r["x-source-ref"].as_str().unwrap().contains("TEC.png"))
            .map(|r| r["title"].as_str().unwrap())
            .collect::<Vec<_>>(),
        expected_hexside
    );
    assert_eq!(terrain_rows.len(), Terrain::ALL.len());
    assert_eq!(hexside_rows.len(), HexsideFeature::ALL.len());
    for (value, name, source) in Terrain::ALL
        .map(|t| {
            (
                serde_json::to_value(t).unwrap(),
                t.row_name(),
                t.source_ref(),
            )
        })
        .into_iter()
        .chain(HexsideFeature::ALL.map(|t| {
            (
                serde_json::to_value(t).unwrap(),
                t.row_name(),
                t.source_ref(),
            )
        }))
    {
        let entry = terrain_rows
            .iter()
            .chain(hexside_rows)
            .find(|r| r["const"] == value)
            .unwrap();
        assert_eq!(entry["title"], name);
        assert_eq!(entry["x-source-ref"], source);
        let (path, section) = source.split_once('#').unwrap();
        assert!(!section.is_empty());
        assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
            .is_file());
    }
}

#[test]
fn terrain_example_roundtrips_all_record_kinds_and_statuses() {
    let value: Value = serde_json::from_str(include_str!(
        "../../../docs/map/examples/terrain-example.json"
    ))
    .unwrap();
    let parsed: Vec<MapRecord> = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), value);
    let values = value.as_array().unwrap();
    assert_eq!(
        values
            .iter()
            .map(|r| r["kind"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        ["terrain", "hexside", "point"].into_iter().collect()
    );
    assert_eq!(
        values
            .iter()
            .map(|r| r["record"]["observation"]["status"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        ["reviewed", "provisional", "unresolved", "fixture"]
            .into_iter()
            .collect()
    );
    let mut missing_provenance = values[0].clone();
    missing_provenance["record"]
        .as_object_mut()
        .unwrap()
        .remove("provenance");
    assert!(serde_json::from_value::<MapRecord>(missing_provenance).is_err());
}

#[test]
fn terrain_unknowns_cannot_become_clear_or_known_minor_cities() {
    for invalid in [
        json!({}),
        json!({"status":"reviewed"}),
        json!({"status":"fixture","value":null}),
        json!({"status":"provisional","value":"unknown"}),
        json!({"status":"unresolved","source_label":"?","value":"clear"}),
    ] {
        assert!(
            serde_json::from_value::<Observation<Terrain>>(invalid.clone()).is_err(),
            "{invalid}"
        );
    }
    let unresolved = json!({"status":"unresolved","source_label":"Minor City"});
    assert!(matches!(
        serde_json::from_value::<Observation<PointFeature>>(unresolved).unwrap(),
        Observation::Unresolved { .. }
    ));
    for status in ["reviewed", "provisional", "fixture"] {
        assert!(serde_json::from_value::<Observation<PointFeature>>(
            json!({"status":status,"value":{"type":"minor_city"}})
        )
        .is_err());
    }
    assert!(serde_json::from_value::<PointFeature>(json!({"type":"port"})).is_err());
    assert!(
        serde_json::from_value::<PointFeature>(json!({"type":"fortification","level":"four"}))
            .is_err()
    );
}

#[test]
fn terrain_directional_sides_reject_silent_orientation_changes() {
    let hex: HexId = "C3617".parse().unwrap();
    let side = HexsideId::new(hex, Direction::East).unwrap();
    assert_eq!(
        serde_json::to_value(side).unwrap(),
        json!({"hex":"C3617","direction":"east"})
    );
    assert_eq!(
        serde_json::from_value::<HexsideId>(serde_json::to_value(side).unwrap()).unwrap(),
        side
    );
    assert!(
        serde_json::from_value::<HexsideId>(json!({"hex":"C3618","direction":"west"})).is_err()
    );
    assert!(
        serde_json::from_value::<HexsideId>(json!({"hex":"C3699","direction":"east"})).is_err()
    );
    assert!(serde_json::from_value::<HexId>(json!("84827")).is_err());
    assert_eq!(
        HexsideFeature::UpSlope.reversed(),
        HexsideFeature::DownSlope
    );
    assert_eq!(
        HexsideFeature::DownEscarpment.reversed(),
        HexsideFeature::UpEscarpment
    );
    assert_eq!(HexsideFeature::Ridge.reversed(), HexsideFeature::Ridge);
    for feature in HexsideFeature::ALL {
        assert_eq!(feature.reversed().reversed(), feature);
    }
}
