use cna_map::hex::{Direction, HexId, HexsideId, MapLocation, MapSection, OffMapBox};
use serde_json::Value;
use std::collections::BTreeSet;

fn census() -> Value {
    serde_json::from_str(include_str!("../../../public/data/map/sources.json")).unwrap()
}

#[test]
fn hex_scenario_ids_and_off_map_names_remain_distinct() {
    for (text, section, col, row) in [
        ("C3926", MapSection::C, 39, 26),
        ("D2228", MapSection::D, 22, 28),
        ("C4218", MapSection::C, 42, 18),
        ("C4120", MapSection::C, 41, 20),
        ("C3617", MapSection::C, 36, 17),
        ("D3714", MapSection::D, 37, 14),
        ("D3612", MapSection::D, 36, 12),
        ("E3815", MapSection::E, 38, 15),
        ("B5925", MapSection::B, 59, 25),
        ("A4829", MapSection::A, 48, 29),
        ("Malta 0805", MapSection::Malta, 8, 5),
    ] {
        let id = HexId::new(section, col, row).unwrap();
        assert_eq!(text.parse::<HexId>(), Ok(id));
        assert_eq!(text.parse::<MapLocation>(), Ok(MapLocation::Hex(id)));
        assert_eq!(id.to_string(), text);
    }
    let sources = census();
    let names: BTreeSet<_> = sources["zones"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|zone| zone["kind"] == "off_map_box")
        .map(|zone| zone["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        OffMapBox::ALL
            .map(OffMapBox::name)
            .into_iter()
            .collect::<BTreeSet<_>>(),
        names
    );
    for name in names {
        let location: MapLocation = name.parse().unwrap();
        assert!(matches!(location, MapLocation::OffMap(_)));
        assert_eq!(location.to_string(), name);
        assert!(name.parse::<HexId>().is_err());
    }
    for invalid in [
        "",
        "84827",
        "1D3612",
        "c4218",
        "F4218",
        "C421",
        "C42180",
        " C4218",
        "C4218 ",
        "C+218",
        "C４２１８",
        "é4218",
        "Malta0805",
        "Malta 805",
        "Map C4218",
        "Weather Track",
        "Tunis-Gabes",
        "Unknown Box",
    ] {
        assert!(invalid.parse::<MapLocation>().is_err(), "{invalid:?}");
    }
    assert!(HexId::new(MapSection::A, 100, 1).is_err());
    assert!(HexId::new(MapSection::A, 1, 100).is_err());
}

#[test]
fn hex_exhaustive_identity_adjacency_and_shared_side_properties() {
    // Exhaustive finite-domain property test: all 60,000 printable identities,
    // including every coordinate boundary and both parities in every section.
    for section in MapSection::ALL {
        for col in 0..=99 {
            for row in 0..=99 {
                let hex = HexId::new(section, col, row).unwrap();
                assert_eq!(hex.to_string().parse::<HexId>(), Ok(hex));
                let mut neighbors = BTreeSet::new();
                for direction in Direction::ALL {
                    if let Some(other) = hex.neighbor(direction) {
                        assert_eq!(other.section(), section, "no inferred seams");
                        assert_ne!(other, hex);
                        assert!(neighbors.insert(other));
                        assert_eq!(other.neighbor(direction.opposite()), Some(hex));
                        let edge = HexsideId::new(hex, direction).unwrap();
                        assert_eq!(edge, HexsideId::new(other, direction.opposite()).unwrap());
                        assert_eq!(edge.hex(), hex.min(other));
                        assert_eq!(edge.hex().neighbor(edge.direction()), Some(hex.max(other)));
                    } else {
                        assert!(HexsideId::new(hex, direction).is_none());
                    }
                }
                if (1..99).contains(&col) && (1..99).contains(&row) {
                    assert_eq!(neighbors.len(), 6);
                }
            }
        }
    }
    let lower: HexId = "A0000".parse().unwrap();
    assert_eq!(lower.neighbor(Direction::SouthEast), None);
    assert_eq!(lower.neighbor(Direction::West), None);
    let upper: HexId = "E9999".parse().unwrap();
    assert_eq!(upper.neighbor(Direction::NorthEast), None);
    assert_eq!(upper.neighbor(Direction::East), None);
}

#[test]
fn hex_adjacency_agrees_with_each_source_numbering_not_just_with_itself() {
    let sources = census();
    for (section, name) in MapSection::ALL
        .into_iter()
        .zip(["Malta", "Map A", "Map B", "Map C", "Map D", "Map E"])
    {
        let zone = sources["zones"]
            .as_array()
            .unwrap()
            .iter()
            .find(|z| z["name"] == name)
            .unwrap();
        let grid = &zone["grid"];
        let numbering = &grid["numbering"];
        assert_eq!(grid["attributes"]["sideways"], "true");
        assert_eq!(numbering["vDescend"], "true");
        assert_eq!(numbering["hDescend"], "false");
        assert_eq!(numbering["first"], "H");
        let ys: Vec<_> = zone["polygon"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p[1].as_i64().unwrap())
            .collect();
        let dx: f64 = grid["attributes"]["dx"].as_str().unwrap().parse().unwrap();
        let max_rows = ((*ys.iter().max().unwrap() - *ys.iter().min().unwrap()) as f64 / dx + 0.5)
            .floor() as i32;
        let h_offset: i32 = numbering["hOff"].as_str().unwrap().parse().unwrap();
        let v_offset: i32 = numbering["vOff"].as_str().unwrap().parse().unwrap();
        let stagger = numbering["stagger"] == "true";
        // Independent lattice projection of VASSAL 3.7.15's numbering rules.
        // Coordinates are in half-widths and row spacings, NOT a pixel geometry API.
        let position = |h: HexId| {
            let raw_column = max_rows - (i32::from(h.col()) - h_offset);
            let parity = raw_column.rem_euclid(2);
            let raw_row = i32::from(h.row()) - v_offset - if stagger { parity } else { 0 };
            (2 * raw_row + parity, raw_column)
        };
        // Symmetry alone cannot detect a flipped convention. Check the expected
        // direction deltas for every interior printed coordinate in each section.
        let deltas = [(2, 0), (1, 1), (-1, 1), (-2, 0), (-1, -1), (1, -1)];
        for col in 1..99 {
            for row in 1..99 {
                let h = HexId::new(section, col, row).unwrap();
                let (x, y) = position(h);
                for (direction, expected) in Direction::ALL.into_iter().zip(deltas) {
                    let (nx, ny) = position(h.neighbor(direction).unwrap());
                    assert_eq!((nx - x, ny - y), expected, "{h} {direction:?}");
                }
            }
        }
    }
}
