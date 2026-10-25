//! Continuous board-pixel geometry from the census and documented image calibration.

use crate::hex::{Direction, HexId, HexsideId, MapSection};
use serde_json::Value;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardPoint {
    pub x: f64,
    pub y: f64,
}

/// Clockwise degrees from screen east, in `Direction::ALL` order.
/// Diagonal = atan2(72.95, 85.25 / 2); not exactly 60° in the supplied grids.
pub const TRAVEL_BEARINGS: [f64; 6] = [
    0.0,
    59.702071576158495,
    120.29792842384151,
    180.0,
    239.7020715761585,
    300.2979284238415,
];

struct Grid {
    dx: f64,
    dy: f64,
    origin: BoardPoint,
    col_base: i32,
    row_offset: i32,
    stagger: bool,
    polygon: Vec<BoardPoint>,
}

fn grids() -> &'static [Grid; 6] {
    static GRIDS: OnceLock<[Grid; 6]> = OnceLock::new();
    GRIDS.get_or_init(|| {
        let source: Value =
            serde_json::from_str(include_str!("../../../public/data/map/sources.json"))
                .expect("checked-in source census must be valid JSON");
        MapSection::ALL.map(|section| {
            let name = if section == MapSection::Malta {
                "Malta".into()
            } else {
                format!("Map {section}")
            };
            let zone = source["zones"]
                .as_array()
                .unwrap()
                .iter()
                .find(|z| z["name"] == name)
                .unwrap();
            let attributes = &zone["grid"]["attributes"];
            let numbering = &zone["grid"]["numbering"];
            // These are invariants of the versioned source, not user inputs.
            assert_eq!(attributes["sideways"], "true");
            assert_eq!(numbering["vDescend"], "true");
            assert_eq!(numbering["hDescend"], "false");
            let number = |key: &str| attributes[key].as_str().unwrap().parse::<f64>().unwrap();
            let integer = |key: &str| numbering[key].as_str().unwrap().parse::<i32>().unwrap();
            let polygon: Vec<_> = zone["polygon"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| BoardPoint {
                    x: p[0].as_f64().unwrap(),
                    y: p[1].as_f64().unwrap(),
                })
                .collect();
            let min_y = polygon.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_y = polygon
                .iter()
                .map(|p| p.y)
                .fold(f64::NEG_INFINITY, f64::max);
            let dx = number("dx");
            let correction = crate::calibration::correction(section);
            Grid {
                dx,
                dy: number("dy"),
                // Sideways exchanges VASSAL's raw x/y after applying its origin.
                origin: BoardPoint {
                    x: number("y0") + correction.x,
                    y: number("x0") + correction.y,
                },
                col_base: ((max_y - min_y) / dx + 0.5).floor() as i32 + integer("hOff"),
                row_offset: integer("vOff"),
                stagger: numbering["stagger"] == "true",
                polygon,
            }
        })
    })
}

impl Grid {
    fn center(&self, hex: HexId) -> BoardPoint {
        let q = self.col_base - i32::from(hex.col());
        let parity = q.rem_euclid(2);
        let r = i32::from(hex.row()) - self.row_offset - if self.stagger { parity } else { 0 };
        BoardPoint {
            x: self.origin.x + (f64::from(r) + f64::from(parity) / 2.0) * self.dy,
            y: self.origin.y + f64::from(q) * self.dx,
        }
    }

    fn contains(&self, point: BoardPoint) -> bool {
        let mut inside = false;
        let mut previous = self.polygon.last().unwrap();
        for next in &self.polygon {
            if (next.y > point.y) != (previous.y > point.y)
                && point.x
                    < (previous.x - next.x) * (point.y - next.y) / (previous.y - next.y) + next.x
            {
                inside = !inside;
            }
            previous = next;
        }
        inside
    }

    fn steps(&self) -> [BoardPoint; 6] {
        [
            BoardPoint { x: self.dy, y: 0.0 },
            BoardPoint {
                x: self.dy / 2.0,
                y: self.dx,
            },
            BoardPoint {
                x: -self.dy / 2.0,
                y: self.dx,
            },
            BoardPoint {
                x: -self.dy,
                y: 0.0,
            },
            BoardPoint {
                x: -self.dy / 2.0,
                y: -self.dx,
            },
            BoardPoint {
                x: self.dy / 2.0,
                y: -self.dx,
            },
        ]
    }
}

/// A lattice center, even if the identity is outside the source polygon.
pub fn center(hex: HexId) -> BoardPoint {
    grids()[hex.section() as usize].center(hex)
}

/// Calibrated center membership in the source section polygon, not playable land.
pub fn contains_hex(hex: HexId) -> bool {
    grids()[hex.section() as usize].contains(center(hex))
}

/// Complete center-in-polygon inventory. Overlapping sections retain distinct IDs.
pub fn hexes() -> &'static [HexId] {
    static HEXES: OnceLock<Vec<HexId>> = OnceLock::new();
    HEXES.get_or_init(|| {
        MapSection::ALL
            .into_iter()
            .flat_map(|section| {
                (0..100).flat_map(move |col| {
                    (0..100).map(move |row| HexId::new(section, col, row).unwrap())
                })
            })
            .filter(|&hex| contains_hex(hex))
            .collect()
    })
}

/// Pointy-top Voronoi corners clockwise, starting at the top of the east edge.
/// This preserves the measured centers and perpendicular shared-edge normals.
pub fn corners(hex: HexId) -> [BoardPoint; 6] {
    let grid = &grids()[hex.section() as usize];
    let c = grid.center(hex);
    let half = grid.dy / 2.0;
    let tip = (grid.dx * grid.dx + half * half) / (2.0 * grid.dx);
    let shoulder = grid.dx - tip;
    [
        (half, -shoulder),
        (half, shoulder),
        (0.0, tip),
        (-half, shoulder),
        (-half, -shoulder),
        (0.0, -tip),
    ]
    .map(|(x, y)| BoardPoint {
        x: c.x + x,
        y: c.y + y,
    })
}

/// The same edge endpoints for either side's canonical `HexsideId`.
pub fn edge(side: HexsideId) -> (BoardPoint, BoardPoint) {
    let vertices = corners(side.hex());
    let index = side.direction() as usize;
    (vertices[index], vertices[(index + 1) % 6])
}

pub fn edge_normal_bearing(hex: HexId, direction: Direction) -> f64 {
    let step = grids()[hex.section() as usize].steps()[direction as usize];
    step.y.atan2(step.x).to_degrees().rem_euclid(360.0)
}

/// Resolve only cells whose centers and the hit lie in a source hex-section polygon.
/// Non-hex boxes/tracks, out-of-board space and nonfinite inputs return `None`.
/// Exact cell-edge ties use the lower `HexId`. This is not a terrain/passability query.
pub fn hex_at(point: BoardPoint) -> Option<HexId> {
    if !point.x.is_finite() || !point.y.is_finite() {
        return None;
    }
    let mut nearest: Option<(f64, HexId)> = None;
    for section in MapSection::ALL {
        let grid = &grids()[section as usize];
        if !grid.contains(point) {
            continue;
        }
        let q_guess = ((point.y - grid.origin.y) / grid.dx).round() as i32;
        // Nine local candidates suffice for this staggered lattice, including boundaries.
        for q in q_guess - 1..=q_guess + 1 {
            let parity = q.rem_euclid(2);
            let r_guess =
                ((point.x - grid.origin.x) / grid.dy - f64::from(parity) / 2.0).round() as i32;
            for r in r_guess - 1..=r_guess + 1 {
                let col = grid.col_base - q;
                let row = r + grid.row_offset + if grid.stagger { parity } else { 0 };
                if !(0..=99).contains(&col) || !(0..=99).contains(&row) {
                    continue;
                }
                let hex = HexId::new(section, col as u8, row as u8).unwrap();
                let c = grid.center(hex);
                if !grid.contains(c) {
                    continue;
                }
                let delta = BoardPoint {
                    x: point.x - c.x,
                    y: point.y - c.y,
                };
                // A clipped-out neighbor must not make a hit jump to a distant cell.
                if grid
                    .steps()
                    .iter()
                    .any(|v| delta.x * v.x + delta.y * v.y > (v.x * v.x + v.y * v.y) / 2.0 + 1e-8)
                {
                    continue;
                }
                let distance = delta.x * delta.x + delta.y * delta.y;
                if nearest.is_none_or(|(best, id)| {
                    distance < best - 1e-8 || ((distance - best).abs() <= 1e-8 && hex < id)
                }) {
                    nearest = Some((distance, hex));
                }
            }
        }
    }
    nearest.map(|(_, hex)| hex)
}
