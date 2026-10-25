//! Canonical map identities and source-derived board geometry.

pub mod calibration;
pub mod fixture;
pub mod geometry;
pub mod hex;
pub mod model;
pub mod terrain;
pub mod viewport;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAP_CONTRACT_VERSION: u32 = 1;
