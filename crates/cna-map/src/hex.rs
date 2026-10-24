//! Printed identities and section-local lattice adjacency, not playable-map membership.
//! See `docs/map/hex-identities.md` for the source-derived numbering convention.

use std::{error::Error, fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum MapSection {
    Malta,
    A,
    B,
    C,
    D,
    E,
}

impl MapSection {
    pub const ALL: [Self; 6] = [Self::Malta, Self::A, Self::B, Self::C, Self::D, Self::E];
}

impl fmt::Display for MapSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Malta => "Malta",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
        })
    }
}

/// The first and second printed digit pairs, not screen x/y coordinates.
/// Both range from 00 to 99. Parsing does not certify that a cell is on the map.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HexId {
    section: MapSection,
    col: u8,
    row: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidMapLocation;

impl fmt::Display for InvalidMapLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "expected A–E followed by four ASCII digits, Malta NNNN, or an exact off-map box name",
        )
    }
}

impl Error for InvalidMapLocation {}

impl HexId {
    pub fn new(section: MapSection, col: u8, row: u8) -> Result<Self, InvalidMapLocation> {
        if col > 99 || row > 99 {
            return Err(InvalidMapLocation);
        }
        Ok(Self { section, col, row })
    }

    pub const fn section(self) -> MapSection {
        self.section
    }

    pub const fn col(self) -> u8 {
        self.col
    }

    pub const fn row(self) -> u8 {
        self.row
    }

    /// Lattice neighbour, possibly outside the section polygon or on sea/unknown terrain.
    /// No off-map movement or unreviewed cross-section connections are inferred.
    pub fn neighbor(self, direction: Direction) -> Option<Self> {
        for &(from, edge, to) in SEAMS {
            if (self, direction) == (from, edge) {
                return Some(to);
            }
            if (self, direction) == (to, edge.opposite()) {
                return Some(from);
            }
        }
        // In every census section, odd printed columns lie half a row west of even ones.
        let east = i16::from(self.col.is_multiple_of(2));
        let (dc, dr) = match direction {
            Direction::East => (0, 1),
            Direction::SouthEast => (-1, east),
            Direction::SouthWest => (-1, east - 1),
            Direction::West => (0, -1),
            Direction::NorthWest => (1, east - 1),
            Direction::NorthEast => (1, east),
        };
        let col = u8::try_from(i16::from(self.col) + dc).ok()?;
        let row = u8::try_from(i16::from(self.row) + dr).ok()?;
        Self::new(self.section, col, row).ok()
    }
}

// E3.I1.S2 owns seam reconciliation. Empty is deliberately not a guessed connection.
const SEAMS: &[(HexId, Direction, HexId)] = &[];

impl fmt::Display for HexId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let separator = if self.section == MapSection::Malta {
            " "
        } else {
            ""
        };
        write!(
            f,
            "{}{separator}{:02}{:02}",
            self.section, self.col, self.row
        )
    }
}

impl FromStr for HexId {
    type Err = InvalidMapLocation;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (section, digits) = if let Some(digits) = value.strip_prefix("Malta ") {
            (MapSection::Malta, digits)
        } else {
            let section = match value.as_bytes().first() {
                Some(b'A') => MapSection::A,
                Some(b'B') => MapSection::B,
                Some(b'C') => MapSection::C,
                Some(b'D') => MapSection::D,
                Some(b'E') => MapSection::E,
                _ => return Err(InvalidMapLocation),
            };
            (section, &value[1..])
        };
        let bytes = digits.as_bytes();
        if bytes.len() != 4 || !bytes.iter().all(u8::is_ascii_digit) {
            return Err(InvalidMapLocation);
        }
        Self::new(
            section,
            (bytes[0] - b'0') * 10 + bytes[1] - b'0',
            (bytes[2] - b'0') * 10 + bytes[3] - b'0',
        )
    }
}

/// Edges clockwise from east in board-image coordinates (screen y increases down).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Direction {
    East,
    SouthEast,
    SouthWest,
    West,
    NorthWest,
    NorthEast,
}

impl Direction {
    pub const ALL: [Self; 6] = [
        Self::East,
        Self::SouthEast,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
        Self::NorthEast,
    ];

    pub const fn opposite(self) -> Self {
        match self {
            Self::East => Self::West,
            Self::SouthEast => Self::NorthWest,
            Self::SouthWest => Self::NorthEast,
            Self::West => Self::East,
            Self::NorthWest => Self::SouthEast,
            Self::NorthEast => Self::SouthWest,
        }
    }
}

/// Shared side stored at the lower-ordered endpoint (section, col, row).
/// Private fields prevent constructing a second identity for the same side.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HexsideId {
    hex: HexId,
    direction: Direction,
}

impl HexsideId {
    pub fn new(hex: HexId, direction: Direction) -> Option<Self> {
        let other = hex.neighbor(direction)?;
        Some(if hex < other {
            Self { hex, direction }
        } else {
            Self {
                hex: other,
                direction: direction.opposite(),
            }
        })
    }

    pub const fn hex(self) -> HexId {
        self.hex
    }

    pub const fn direction(self) -> Direction {
        self.direction
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum OffMapBox {
    Sicily,
    Italy,
    Benghazi,
    Derna,
    Crete,
    Bardia,
    Tobruk,
    Alexandria3613,
    Alexandria3714,
    Tunis,
    Gabes,
    Tripoli,
    Tripolitania,
    GermanShipping,
}

impl OffMapBox {
    pub const ALL: [Self; 14] = [
        Self::Sicily,
        Self::Italy,
        Self::Benghazi,
        Self::Derna,
        Self::Crete,
        Self::Bardia,
        Self::Tobruk,
        Self::Alexandria3613,
        Self::Alexandria3714,
        Self::Tunis,
        Self::Gabes,
        Self::Tripoli,
        Self::Tripolitania,
        Self::GermanShipping,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Sicily => "Sicily Holding Box",
            Self::Italy => "Italy Holding Box",
            Self::Benghazi => "Benghazi Holding Box",
            Self::Derna => "Derna Holding Box",
            Self::Crete => "Crete Holding Box",
            Self::Bardia => "Bardia Holding Box",
            Self::Tobruk => "Tobruk Holding Box",
            Self::Alexandria3613 => "Alexandria Holding Box Hex 3613",
            Self::Alexandria3714 => "Alexandria  Holding Box Hex 3714",
            Self::Tunis => "Tunis",
            Self::Gabes => "Gabes",
            Self::Tripoli => "Tripoli",
            Self::Tripolitania => "Tripolitania",
            Self::GermanShipping => "German Shipping",
        }
    }
}

impl fmt::Display for OffMapBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for OffMapBox {
    type Err = InvalidMapLocation;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|b| b.name() == value)
            .ok_or(InvalidMapLocation)
    }
}

/// Named boxes never acquire hex neighbours merely by parsing a location string.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum MapLocation {
    Hex(HexId),
    OffMap(OffMapBox),
}

impl fmt::Display for MapLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hex(hex) => hex.fmt(f),
            Self::OffMap(off_map) => off_map.fmt(f),
        }
    }
}

impl FromStr for MapLocation {
    type Err = InvalidMapLocation;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse()
            .map(Self::Hex)
            .or_else(|_| value.parse().map(Self::OffMap))
    }
}
