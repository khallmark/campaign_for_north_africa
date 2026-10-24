# Map v1 contract — identity and board geometry

This is the developing E1 contract, **not yet the published map-contract-v1
fixture/tag**. Serialization, terrain records, viewport transforms and the
consumer fixture remain separate leaves. No source-art redistribution permission
is implied: see [source census](../map/sources.md) and
[licensing and release guard](../map/licensing.md).

## Identity and direction

Use `cna_map::hex::{HexId, MapSection, Direction, HexsideId, MapLocation,
OffMapBox}`. The strict printed-ID grammar, offsets and section-local neighbour
contract are in [hex identities](../map/hex-identities.md). IDs and lattice
neighbours do not certify playable terrain. Cross-section seams are not inferred.

The sideways VASSAL grids are **pointy-top**, with six edge-normal directions
clockwise from image east. Image x increases right and y increases down. Printed
columns increase north, not right; printed rows increase east.

```text
                  top corner
                      /\
          NW         /  \         NE
                    /    \
                   /      \
         W        | center |        E = edge 0
                   \      /
                    \    /
          SW         \  /         SE
                      \/
                 bottom corner

Clockwise edges: E, SE, SW, W, NW, NE
Corners start at the upper end of edge E and run clockwise.
```

## Board-pixel API

`cna_map::geometry` exposes:

| Function/type | Contract |
| --- | --- |
| `BoardPoint { x, y }` | Continuous `f64` pixels in the full 14310 × 4632 source board; independent of screen zoom/DPR. |
| `center(HexId)` | Lattice center from the section's recorded grid and numbering parameters, even for a syntactic ID outside its polygon. |
| `corners(HexId)` | Six clockwise Voronoi corners, beginning at the upper end of the east-facing edge. |
| `edge(HexsideId)` | Endpoints of the canonical shared side; either incident hex constructs the same side ID. |
| `edge_normal_bearing(HexId, Direction)` | Outward normal in degrees clockwise from screen east, toward that lattice neighbour. |
| `TRAVEL_BEARINGS` | E/SE/SW/W/NW/NE: 0°, 59.702071576158495°, 120.29792842384151°, 180°, 239.7020715761585°, 300.2979284238415°. |
| `hex_at(BoardPoint)` | Nearest lattice cell when both hit and center lie inside a source hex-section polygon and the hit lies in that cell. `None` for boxes/tracks, outside space, and nonfinite points. |

Exact shared-edge ties use the lower `HexId`. Polygon clipping follows a half-open
ray crossing convention; this is a provisional census boundary, not certified
coast/sea terrain. Geometry does not establish adjacency across section seams.

## Continuous geometry preserves the recorded spacings

For raw column `q` and raw row `r` derived in the identity note, the source's
sideways axis swap gives:

```text
x = y0 + (r + (q mod 2)/2) * dy
y = x0 + q * dx
```

There is no translation by the zone's minimum coordinates: its polygon is already
in board pixels. The polygon height affects numbering, not origin translation.
Unlike VASSAL's Java integer pixel lookup, the contract retains fractional pixels;
independent anchors/calibration are E3.I1.S1, not completed by this API.

All six measured grids use `dx=72.95`, `dy=85.25`. These are not the spacings of an
exact regular hexagon (`dx` would be about 73.829). Forcing 60° normals or using
the VASSAL drawing's stretched regular-hex corners would contradict the required
perpendicularity to center-to-center travel. The contract therefore uses the
**Voronoi cell**: edges are perpendicular bisectors of adjacent centers. For
`h=dy/2`, its top/bottom radius is `(dx²+h²)/(2dx)` and the east/west edge's
half-height is `dx − radius`. Adjacent cells share the same segment exactly.
This small shape difference is explicit; matching artwork is later calibration.

Tests exercise both sides of every edge for asymmetric fixture IDs across all six
sections, exact numeric centers, outward/perpendicular normals, shared endpoint
identity, constant bearings, deterministic ties, and rejection of every non-hex
zone's representative center. Run `cargo test --locked -p cna-map geometry`.
