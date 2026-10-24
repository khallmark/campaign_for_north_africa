use cna_map::{
    calibration::ANCHORS,
    geometry::{center, hex_at},
    hex::{HexId, MapSection},
};
use std::collections::BTreeMap;

#[test]
fn calibration_centers_match_independent_image_observations_within_three_pixels() {
    let mut counts = BTreeMap::new();
    for (id, observed) in ANCHORS {
        let hex: HexId = id.parse().unwrap();
        *counts.entry(hex.section()).or_insert(0) += 1;
        let predicted = center(hex);
        let residual = (predicted.x - observed[0]).hypot(predicted.y - observed[1]);
        assert!(residual <= 3.0, "{id}: {residual} board px");
        assert_eq!(hex_at(predicted), Some(hex), "{id}");
    }
    for section in MapSection::ALL {
        assert!(counts[&section] >= 5);
    }
    // Explicit independently evaluated output protects the correction sign and
    // keeps calibration in both forward geometry and inverse hit testing.
    let buq = center("C3926".parse().unwrap());
    assert!((buq.x - 7855.25).abs() < 1e-9);
    assert!((buq.y - 1734.8).abs() < 1e-9);
}
