//! CNA engine boundaries. The playable prototype is explicitly a sandbox.

pub mod contract;
pub mod official;
pub mod sandbox;
pub mod tabletop;

// Compatibility for the existing browser; official rules must not use these.
pub use sandbox::*;
