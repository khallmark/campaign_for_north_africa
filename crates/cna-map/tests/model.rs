use cna_map::{
    geometry::{center, contains_hex, hexes},
    hex::{Direction, HexId, MapSection},
    model::{model, Model, SourcePointStatus, SourceSymbol},
    terrain::{MapRecord, Observation, TransportDepiction},
};
use std::collections::BTreeSet;

#[test]
fn complete_section_inventory_preserves_overlapping_identities_and_excludes_offboard_ids() {
    let members: BTreeSet<_> = hexes().iter().copied().collect();
    assert_eq!(members.len(), hexes().len());
    // Pinned calibrated polygon census, including Malta and raster edge cells.
    for (section, count) in MapSection::ALL
        .into_iter()
        .zip([67, 1013, 1989, 1545, 1253, 1313])
    {
        assert_eq!(
            members.iter().filter(|h| h.section() == section).count(),
            count
        );
        for col in 0..100 {
            for row in 0..100 {
                let hex = HexId::new(section, col, row).unwrap();
                assert_eq!(members.contains(&hex), contains_hex(hex));
            }
        }
    }
    let a: HexId = "A3033".parse().unwrap();
    let b: HexId = "B3000".parse().unwrap();
    assert!(members.contains(&a) && members.contains(&b));
    assert!((center(a).x - center(b).x).abs() < 3.0);
    assert!(Direction::ALL.into_iter().all(|d| a.neighbor(d) != Some(b)));
    for id in ["A0000", "C9999", "D3000"] {
        assert!(!members.contains(&id.parse().unwrap()), "{id}");
    }
}

#[test]
fn source_surveys_expand_into_map_v1_without_promoting_uncertainty() {
    model().validate().unwrap();
    let records: Vec<_> = model().surveys.iter().flat_map(|s| s.records()).collect();
    let bytes = serde_json::to_vec(&records).unwrap();
    assert_eq!(
        serde_json::from_slice::<Vec<MapRecord>>(&bytes).unwrap(),
        records
    );
    let port = model()
        .surveys
        .iter()
        .find(|s| s.id == "matruh-port-location")
        .unwrap()
        .records();
    assert!(matches!(&port[0], MapRecord::Point { hex, record }
        if hex.to_string() == "D3814" && matches!(record.observation, Observation::Unresolved { .. })));
    let unfinished = model()
        .surveys
        .iter()
        .find(|s| s.id == "matruh-rail-west")
        .unwrap()
        .records();
    assert!(!unfinished.is_empty());
    assert!(unfinished.iter().all(|r| matches!(r, MapRecord::Hexside {
        depiction: Some(TransportDepiction::UnfinishedRailroad), record, ..
    } if matches!(record.observation, Observation::Unresolved { .. }))));
    // A parseable offboard location and a duplicate assignment must fail import.
    let original = serde_json::to_value(model()).unwrap();
    let mut contradictory = original.clone();
    let survey = contradictory["surveys"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["id"] == "matruh-rail-west")
        .unwrap();
    survey["coverage"]["observation"] =
        serde_json::json!({"status":"provisional","value":"railroad"});
    assert!(serde_json::from_value::<Model>(contradictory)
        .unwrap()
        .validate()
        .is_err());
    for locations in [
        serde_json::json!(["C9999"]),
        serde_json::json!(["D3714", "D3714"]),
    ] {
        let mut changed = original.clone();
        changed["surveys"][0]["coverage"]["hexes"] = locations;
        assert!(serde_json::from_value::<Model>(changed)
            .unwrap()
            .validate()
            .is_err());
    }
    let mut unpinned = original;
    unpinned["sources"] = serde_json::json!({"other":"a".repeat(64)});
    assert!(serde_json::from_value::<Model>(unpinned)
        .unwrap()
        .validate()
        .is_err());
}

#[test]
fn source_locations_keep_measured_symbols_labels_and_uncertainty() {
    let hex: HexId = "C3111".parse().unwrap();
    let points = &model().examinations[&hex].points;
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].symbol, SourceSymbol::VillageBirDot);
    assert_eq!(points[0].center, [6567.0, 2324.1]);
    assert_eq!(points[0].label.as_deref(), Some("El Cuasc"));
    assert_eq!(points[0].status, SourcePointStatus::Unresolved);
    assert_eq!(points[0].source_refs.len(), 2);
    assert!(matches!(
        &model()
            .surveys
            .iter()
            .find(|s| s.id == "label-C3111")
            .unwrap()
            .coverage,
        cna_map::model::Coverage::Point {
            observation: Observation::Unresolved { .. },
            ..
        }
    ));
    let original = serde_json::to_value(model()).unwrap();
    for (field, invalid) in [
        ("center", serde_json::json!([6600.0, 2400.0])),
        ("label", serde_json::json!(" ")),
        ("source_refs", serde_json::json!([])),
        (
            "source_refs",
            serde_json::json!(["untracked.png#xywh=1,2,3,4"]),
        ),
    ] {
        let mut changed = original.clone();
        changed["examinations"]["C3111"]["points"][0][field] = invalid;
        assert!(
            serde_json::from_value::<Model>(changed)
                .unwrap()
                .validate()
                .is_err(),
            "{field}"
        );
    }
}
