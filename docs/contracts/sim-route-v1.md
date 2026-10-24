# Accepted route v1

`cna_sim::contract::route::RouteResult` is the accepted route boundary for movement
playback. It is not a terrain-cost calculator or a claim of official rules support.
Its `HexId` values and adjacency come from `cna-map` contract v1, not sandbox hexes.

## Steps, costs and events

- `command_id` identifies the accepted command; `revision` is its resulting
  authoritative revision. `unit_ref` is the instance ID, not a catalog type ID.
- `steps[0]` is the origin with zero incremental and cumulative costs. A four-leg
  route therefore has five steps. `steps[i]` for i > 0 is the reached destination
  of leg i. `cp` and `fuel` are that leg's costs; `cumulative` contains total
  `cp` and `fuel` from the origin through that destination.
- Each cost is exact nonnegative points as `{numerator: u32, denominator: u32}`;
  denominator must be nonzero. Producer values use reduced fractions, with zero
  written 0/1. This avoids rounding fractional costs or conflating CP and fuel.
  Real costs and their arithmetic are later rule-producer work (CNA-323).
- `stop.kind` is `completed`, `zoc`, `reaction`, `minefield`, `breakdown`, or
  `rejected_suffix`. `stop.at_step` is the zero-based u32 index of the final reached
  step. Only the accepted prefix is present: a rejected or interrupted suffix
  is never animated, revealed, or charged. A stop before travel has only origin.
  A wholly rejected command uses the command rejection envelope, not a route.
- `reveals` is ordered by zero-based `at_step`; multiple deltas at one step keep
  array order. Resolve `projection_delta_ref` through the authorized projection
  stream, never by reading authoritative state. Apply each delta only once that
  center is reached; no look-ahead. Reduced motion still processes deltas in order.

Call `RouteResult::validate()` before playback. It rejects blank identifiers,
empty routes, non-adjacent consecutive cells, a stop not at the last provided
step, and invalid/out-of-order reveal indices. Serde rejects unknown fields and
zero denominators. These are structural checks, not validation of costs,
map membership, command authority, reference visibility or historical terrain.
The authoritative producer owns those checks and must emit correct cumulative
totals; the browser must never recompute costs from terrain.

For each adjacent pair, obtain its direction via `HexId::neighbor`, then ask
`cna_map::geometry::edge_normal_bearing` for its bearing and `center` for endpoints.
Consumers never recompute bearings with trigonometry, assume exact multiples of
60°, or substitute sandbox coordinates. Use map `Viewport` for CSS projection.
The route contains no duration, sprite frame, DOM or screen coordinate fields.

## Fixture decisions and limits

Every `fixtures/route-*.json` wraps the route with `status: fixture` and
`costs: invented_non_certified`. Both fields are mandatory and typed. All cells
belong to the current 44-cell corridor fixture, but costs are invented independent
of that fixture's synthetic terrain. Never include these fixtures in certified
scenarios. `fixture-unit-1` and projection delta IDs are synthetic references;
CNA-161/CNA-4 must bind them to their fixture projection, not historical units.

| File | Required consumer behavior |
| --- | --- |
| `route-straight.json` | Four eastbound legs C3922→C3926; asymmetric fractional CP/fuel totals |
| `route-three-turn.json` | Four legs with three heading changes; complete each turn before translation |
| `route-seam.json` | NE→E→SE crosses the 360°/0° heading seam via the short arc |
| `route-zoc.json` | Stop at reached step 2; no unaccepted suffix exists in the payload |
| `route-reveal.json` | Resolve `fixture-reveal-step-3` only on reaching step 3 |
| `route-malformed.json` | Reject the non-adjacent C3922→C4321 jump before any playback |

The seam fixture means the **angular seam**, not a map-section seam. Map v1's
cross-section seam table is deliberately empty pending reconciliation; no route
fixture invents a cross-section connection. Off-map/unknown cells remain unknown.

`cargo test --locked -p cna-sim contract::route` checks fixture status, corridor
membership, exact straight/turn/seam/stop/reveal shapes, rejection of the malformed
route, invalid indices, zero denominators and certification relabeling. Real
movement legality, interruption causes, visibility timing in a rendered UI and
official costs require their later producers/consumers; these tests prove none
of those behaviors yet.
