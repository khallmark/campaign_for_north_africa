//! Image-measured grid centers. See docs/map/calibration.md for method and limits.
use crate::{geometry::BoardPoint, hex::MapSection};

pub const SOURCE_SHA256: &str = "e82f995e213da486004d006e5b7fd1f6e49e7464fb7fd2db23f36bbbc57b14b4";

/// Median measured translation per section, in board pixels. Raw census is unchanged.
pub const fn correction(section: MapSection) -> BoardPoint {
    let (x, y) = match section {
        MapSection::Malta => (0.25, 0.0),
        MapSection::A => (1.25, 2.25),
        MapSection::B => (2.0, 0.5),
        MapSection::C => (2.25, 0.0),
        MapSection::D => (-0.75, -0.25),
        MapSection::E => (0.0, 0.5),
    };
    BoardPoint { x, y }
}

/// Independently measured from source-image line alignment at quarter-pixel steps.
/// These are observations, never recomputed from `geometry::center` by tests.
pub const ANCHORS: [(&str, [f64; 2]); 30] = [
    ("Malta 0602", [302.375, 641.3]),
    ("Malta 1003", [387.625, 349.75]),
    ("Malta 0805", [558.375, 495.4]),
    ("Malta 0906", [601.0, 422.7]),
    ("Malta 1204", [473.125, 203.85]),
    ("A4829", [2521.5, 1078.8]),
    ("A4130", [2564.375, 1589.2]),
    ("A4929", [2478.625, 1005.6]),
    ("A3028", [2436.25, 2390.9]),
    ("A2028", [2436.25, 3119.9]),
    ("B4827", [5167.625, 1078.75]),
    ("B5925", [4954.25, 276.8]),
    ("B4921", [4612.5, 1005.8]),
    ("B4020", [4569.875, 1662.1]),
    ("B3020", [4569.875, 2390.85]),
    ("C3926", [7856.25, 1734.8]),
    ("C4321", [7429.5, 1443.25]),
    ("C4807", [6277.125, 1078.75]),
    ("C1014", [6874.625, 3848.85]),
    ("C3617", [7130.625, 1953.4]),
    ("D3714", [9648.5, 1880.7]),
    ("D3612", [9520.625, 1953.4]),
    ("D3615", [9776.625, 1953.4]),
    ("D2020", [10203.625, 3120.1]),
    ("D3010", [9349.625, 2390.85]),
    ("E3815", [12593.625, 1807.75]),
    ("E1829", [13788.625, 3265.75]),
    ("E1430", [13873.625, 3557.3]),
    ("E2015", [12593.375, 3119.85]),
    ("E3015", [12593.375, 2390.85]),
];
