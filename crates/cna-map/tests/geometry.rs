use cna_map::geometry::{
    center, corners, edge, edge_normal_bearing, hex_at, BoardPoint, TRAVEL_BEARINGS,
};
use cna_map::hex::{Direction, HexId, HexsideId};

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

#[test]
fn geometry_centers_use_section_offsets_and_swapped_axes() {
    // Independently evaluated from the source numbers, not from another API call.
    for (id, x, y) in [
        ("Malta 0805", 558.125, 495.65),
        ("A4829", 2520.25, 1076.3),
        ("A4929", 2477.625, 1003.35),
        ("B5925", 4951.5, 275.8),
        ("C4218", 7213.625, 1515.95),
        ("D3714", 9649.25, 1880.7),
        ("E3815", 12593.625, 1806.75),
    ] {
        let hex: HexId = id.parse().unwrap();
        let c = center(hex);
        close(c.x, x);
        close(c.y, y);
        assert_eq!(hex_at(BoardPoint { x, y }), Some(hex), "{id}");
    }
}

#[test]
fn geometry_shared_edges_normals_and_hit_testing_agree_on_both_sides() {
    for id in [
        "Malta 0805",
        "A4829",
        "A4929",
        "B5925",
        "C4218",
        "C4120",
        "C3617",
        "D3714",
        "D3612",
        "E3815",
    ] {
        let hex: HexId = id.parse().unwrap();
        let c = center(hex);
        let vertices = corners(hex);
        for (index, direction) in Direction::ALL.into_iter().enumerate() {
            let neighbor = hex.neighbor(direction).unwrap();
            let n = center(neighbor);
            let side = HexsideId::new(hex, direction).unwrap();
            let (a, b) = edge(side);
            let expected_a = vertices[index];
            let expected_b = vertices[(index + 1) % 6];
            // Canonical side order may be reversed, but the segment is identical.
            let (first, second) = if hex < neighbor { (a, b) } else { (b, a) };
            close(first.x, expected_a.x);
            close(first.y, expected_a.y);
            close(second.x, expected_b.x);
            close(second.y, expected_b.y);
            let travel = BoardPoint {
                x: n.x - c.x,
                y: n.y - c.y,
            };
            let length = travel.x.hypot(travel.y);
            let normal = BoardPoint {
                x: travel.x / length,
                y: travel.y / length,
            };
            close(normal.x * (b.x - a.x) + normal.y * (b.y - a.y), 0.0);
            let bearing = edge_normal_bearing(hex, direction);
            close(bearing, TRAVEL_BEARINGS[index]);
            close(bearing.to_radians().cos(), normal.x);
            close(bearing.to_radians().sin(), normal.y);
            let midpoint = BoardPoint {
                x: (a.x + b.x) / 2.0,
                y: (a.y + b.y) / 2.0,
            };
            close(midpoint.x, (c.x + n.x) / 2.0);
            close(midpoint.y, (c.y + n.y) / 2.0);
            assert_eq!(
                hex_at(BoardPoint {
                    x: midpoint.x - normal.x,
                    y: midpoint.y - normal.y
                }),
                Some(hex),
                "inside {id} {direction:?}"
            );
            assert_eq!(
                hex_at(BoardPoint {
                    x: midpoint.x + normal.x,
                    y: midpoint.y + normal.y
                }),
                Some(neighbor),
                "outside {id} {direction:?}"
            );
            assert_eq!(
                hex_at(midpoint),
                Some(hex.min(neighbor)),
                "edge tie {id} {direction:?}"
            );
        }
    }
}

#[test]
fn geometry_rejects_non_hex_regions_and_nonfinite_points() {
    for (x, y) in [
        (0.0, 0.0),
        (-100.0, 200.0),
        (14311.0, 4633.0),
        (13070.0, 245.0),
        (f64::NAN, 200.0),
        (100.0, f64::INFINITY),
        (f64::MAX, f64::MAX),
    ] {
        assert_eq!(hex_at(BoardPoint { x, y }), None);
    }
    // Source holding boxes and tracks must not become selectable hexes.
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../../../public/data/map/sources.json")).unwrap();
    for zone in source["zones"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|z| z["kind"] != "hex_section")
    {
        let polygon = zone["polygon"].as_array().unwrap();
        let count = polygon.len() as f64;
        let p = BoardPoint {
            x: polygon.iter().map(|p| p[0].as_f64().unwrap()).sum::<f64>() / count,
            y: polygon.iter().map(|p| p[1].as_f64().unwrap()).sum::<f64>() / count,
        };
        assert_eq!(hex_at(p), None, "{}", zone["name"]);
    }
}
