//! Accepted route wire contract, not a movement-cost producer.
use cna_map::hex::{Direction, HexId};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Exact nonnegative points; fractional costs must not be rounded for display.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Points {
    pub numerator: u32,
    pub denominator: NonZeroU32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CumulativeCost {
    pub cp: Points,
    pub fuel: Points,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteStep {
    pub hex: HexId,
    pub cp: Points,
    pub fuel: Points,
    pub cumulative: CumulativeCost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopKind {
    Completed,
    Zoc,
    Reaction,
    Minefield,
    Breakdown,
    RejectedSuffix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    pub kind: StopKind,
    pub at_step: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reveal {
    pub at_step: u32,
    pub projection_delta_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteResult {
    pub command_id: String,
    pub revision: u64,
    pub unit_ref: String,
    pub steps: Vec<RouteStep>,
    pub stop: Stop,
    pub reveals: Vec<Reveal>,
}

impl RouteResult {
    /// Structural consumer check only. Does not certify costs, terrain, authority,
    /// visibility or membership in a loaded map. Call before starting playback.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.command_id.trim().is_empty() || self.unit_ref.trim().is_empty() {
            return Err("route identifiers must not be blank");
        }
        if self.steps.is_empty() || self.stop.at_step as usize != self.steps.len() - 1 {
            return Err("route must end at its stop index and include its origin");
        }
        for pair in self.steps.windows(2) {
            if !Direction::ALL
                .into_iter()
                .any(|d| pair[0].hex.neighbor(d) == Some(pair[1].hex))
            {
                return Err("route contains non-adjacent consecutive hexes");
            }
        }
        if self.reveals.iter().any(|r| {
            r.at_step as usize >= self.steps.len() || r.projection_delta_ref.trim().is_empty()
        }) || self
            .reveals
            .windows(2)
            .any(|pair| pair[0].at_step > pair[1].at_step)
        {
            return Err("route reveals must reference reached steps in order");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureStatus {
    Fixture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureCosts {
    InventedNonCertified,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteFixture {
    pub status: FixtureStatus,
    pub costs: FixtureCosts,
    pub route: RouteResult,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_map::{geometry::edge_normal_bearing, terrain::MapRecord};
    use std::collections::BTreeSet;

    const FIXTURES: [&str; 6] = [
        include_str!("../../fixtures/route-straight.json"),
        include_str!("../../fixtures/route-three-turn.json"),
        include_str!("../../fixtures/route-seam.json"),
        include_str!("../../fixtures/route-zoc.json"),
        include_str!("../../fixtures/route-reveal.json"),
        include_str!("../../fixtures/route-malformed.json"),
    ];

    fn fixture(index: usize) -> RouteFixture {
        serde_json::from_str(FIXTURES[index]).unwrap()
    }

    #[test]
    fn route_fixtures_use_corridor_neighbors_and_reject_the_malformed_route() {
        let corridor: BTreeSet<_> = cna_map::fixture::corridor()
            .into_iter()
            .filter_map(|r| {
                if let MapRecord::Terrain { hex, .. } = r {
                    Some(hex)
                } else {
                    None
                }
            })
            .collect();
        for i in 0..FIXTURES.len() {
            let f = fixture(i);
            assert_eq!(f.status, FixtureStatus::Fixture);
            assert_eq!(f.costs, FixtureCosts::InventedNonCertified);
            assert!(f.route.steps.iter().all(|s| corridor.contains(&s.hex)));
            assert_eq!(f.route.validate().is_ok(), i != 5);
        }
        let straight = fixture(0);
        assert_eq!(straight.route.steps.len(), 5);
        assert!(straight
            .route
            .steps
            .windows(2)
            .all(|p| p[0].hex.neighbor(Direction::East) == Some(p[1].hex)));
        let headings = |route: RouteResult| {
            route
                .steps
                .windows(2)
                .map(|p| {
                    let d = Direction::ALL
                        .into_iter()
                        .find(|&d| p[0].hex.neighbor(d) == Some(p[1].hex))
                        .unwrap();
                    edge_normal_bearing(p[0].hex, d)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            headings(fixture(1).route)
                .windows(2)
                .filter(|p| p[0] != p[1])
                .count(),
            3
        );
        let seam = headings(fixture(2).route);
        assert!(seam[0] > 300.0 && seam[0] < 301.0);
        assert_eq!(seam[1], 0.0);
        assert_eq!(
            fixture(3).route.stop,
            Stop {
                kind: StopKind::Zoc,
                at_step: 2
            }
        );
        assert_eq!(fixture(4).route.reveals[0].at_step, 3);
    }

    #[test]
    fn route_boundary_rejects_invalid_indices_and_fixture_certification() {
        let mut route = fixture(0).route;
        route.stop.at_step = 2;
        assert!(route.validate().is_err());
        route.stop.at_step = 4;
        route.reveals.push(Reveal {
            at_step: 5,
            projection_delta_ref: "fixture-delta".into(),
        });
        assert!(route.validate().is_err());
        route.reveals = vec![
            Reveal {
                at_step: 1,
                projection_delta_ref: "first".into(),
            },
            Reveal {
                at_step: 1,
                projection_delta_ref: "second".into(),
            },
            Reveal {
                at_step: 3,
                projection_delta_ref: "third".into(),
            },
        ];
        assert!(
            route.validate().is_ok(),
            "same-step deltas retain array order"
        );
        let restored: RouteResult =
            serde_json::from_str(&serde_json::to_string(&route).unwrap()).unwrap();
        assert_eq!(restored.reveals, route.reveals);
        route.reveals.swap(0, 2);
        assert!(
            route.validate().is_err(),
            "a later reveal cannot precede an earlier one"
        );
        route.reveals.clear();
        route.steps.truncate(1);
        route.stop = Stop {
            kind: StopKind::RejectedSuffix,
            at_step: 0,
        };
        assert!(
            route.validate().is_ok(),
            "an accepted origin-only stop has no travel"
        );
        route.steps.clear();
        assert!(route.validate().is_err());
        assert!(serde_json::from_str::<RouteFixture>(
            &FIXTURES[0].replace("\"fixture\"", "\"verified\"")
        )
        .is_err());
        assert!(serde_json::from_str::<RouteFixture>(
            &FIXTURES[0].replace("\"denominator\":1", "\"denominator\":0")
        )
        .is_err());
    }
}
