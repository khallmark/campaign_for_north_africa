# Map v1 contract — identity and board geometry

Contract identifier **map-contract-v1**, `cna_map::MAP_CONTRACT_VERSION = 1`.
The versioned consumer fixture is available locally; no remote tag or release
has been published. No source-art redistribution permission
is implied: see [source census](../map/sources.md) and
[licensing and release guard](../map/licensing.md).

## Identity and direction

Use `cna_map::hex::{HexId, MapSection, Direction, HexsideId, MapLocation,
OffMapBox}`. The strict VASSAL-derived ID grammar, offsets and section-local
neighbour contract are in [hex identities](../map/hex-identities.md). These labels
are not verified original printed aliases; no reviewed numbering bridge exists.
IDs and lattice neighbours do not certify playable terrain. Cross-section seams
are not inferred, and section-local `HexId` and occupancy semantics are unchanged.

Source-image calibration proves alignment, not equivalence to the original
rulebook's numbering. VASSAL seam candidates, if recorded, must be separately
versioned and source-hash-pinned, with residuals and original-rule numbering
explicitly unresolved. They must not deduplicate official occupancy or enable
official movement. CNA-20's official/canonical seam acceptance remains gated on
reviewed bridging evidence; a VASSAL-only candidate result cannot complete it.

The sideways VASSAL grids are **pointy-top**, with six edge-normal directions
clockwise from image east. Image x increases right and y increases down. VASSAL
columns increase north, not right; VASSAL rows increase east.

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
| `center(HexId)` | Lattice center from the section's recorded grid and numbering parameters plus its documented image-calibration translation, even for a syntactic ID outside its polygon. |
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
the formula above describes the raw census position. Add the section translation
in [calibration](../map/calibration.md) for the calibrated position used by all
geometry APIs, including inverse hit testing. Raw census data stays unchanged.
The 30 measured anchors validate image alignment, not terrain or section seams.

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

## Viewport coordinates are CSS pixels, independent of DPR

`viewport::Viewport::new(center, zoom, css_size, device_pixel_ratio)` accepts a
board-pixel center, CSS pixels per board pixel, `[width, height]` in CSS pixels,
and DPR. Coordinates must be finite; size, zoom and DPR must be positive.
`to_screen(BoardPoint)` returns a distinct `ScreenPoint` in CSS pixels relative
to the viewport's upper-left corner. `to_board(ScreenPoint)` is its inverse.
The frontend must subtract the element's client-rectangle origin first. Multiply
CSS pixels by DPR only when addressing a raster backing store, not for hit tests.

`zoom_about(screen_point, new_zoom)` keeps the selected board point fixed under
that CSS coordinate. It deliberately does not clamp: anchoring and boundary
clamping conflict at a board edge. Call `clamp_to_board([width, height])`
explicitly when boundary policy takes priority. Clamping keeps the visible span
inside the board on each axis; an oversized span is centered with symmetric
margins. No tiles, DOM inputs, resize observers or playback logic live here.

Viewport checks cover the required 2458×1358 / 1280×800 sizes, zooms 0.05 / 1 / 4,
and DPR 1 / 2; 10,000 generated asymmetric zoom-anchor cases; both board edges;
and mixed-axis letterboxing. Run `cargo test --locked -p cna-map viewport`.

## Geography records preserve evidence status

`terrain::MapRecord` distinguishes terrain, canonical hexside and point-feature
records. Each carries typed observation status, provenance and an optional
adjudication ID. Hex IDs serialize as VASSAL-derived label strings; sides serialize as
`{"hex":"C3617","direction":"east"}` and must already be canonical.
See [geography schema](../map/map-v1.schema.json) and
[source vocabulary, errata and unresolved labels](../map/terrain-schema.md).
Unknown observations have only `status=unresolved` and a source label, never an
implicit Clear terrain value. The requested but currently unsourced Minor City
category remains in that unresolved representation. No costs or real terrain
assignments are supplied by the schema.

## Corridor consumer fixture

`cna_map::fixture::corridor() -> Vec<MapRecord>` loads the embedded
`crates/cna-map/fixtures/corridor-fixture.json`. It contains 44 connected Map C
identities in the C30xx–C43xx corridor, including C3926 and C4321, plus synthetic
escarpment, road, track and coast sides. All 48 records carry `status=fixture`.
The selected strip exercises the Sidi Barrani–Buq Buq–Sollum corridor contract;
it is not a transcription of the towns, coastline or roads. Terrain choices
(Clear/Desert/Rough) and all four side assignments are invented test inputs.
Their provenance points here deliberately, not to a claim of historical review.

Geometry comes exclusively from the source-derived S2 API above: call
`geometry::center(hex)`, `corners(hex)` and `edge(side)` on the loaded identities.
The JSON does not duplicate coordinates that could drift from that producer.
Every sampled center and all six of its neighbours resolve inside the census
section polygon. There are no board edges in this sample. A neighbour absent
from this small fixture is **unknown geography**, not a board edge, impassable
terrain, or an implied Clear cell. Section seams remain unimplemented.

**Consumers must reject fixture observations when certifying a scenario.**
Do not relabel them as reviewed, fill missing terrain with Clear, or derive
historical movement costs from their invented assignments. This leaf defines
neither a certified-scenario validator nor movement arithmetic.

```sh
cargo test --locked -p cna-map fixture
uvx check-jsonschema --schemafile docs/map/map-v1.schema.json crates/cna-map/fixtures/corridor-fixture.json
```

## Changelog

- **2026-10-24 / CNA-20 adjudication:** clarified VASSAL-derived versus original
  printed identities. No identity, occupancy, adjacency or serialization behavior
  changed; the seam table stays empty pending reviewed bridging evidence.
- **2026-10-24 / CNA-19:** measured section translations added consistently to
  forward and inverse geometry. Consumers must use the geometry API rather than
  cache pre-calibration centers. Identity, spacing and bearings are unchanged;
  the original local `map-contract-v1` checkpoint tag is not moved.
- **v1 (2026-10-24):** VASSAL-derived IDs and canonical sides, census geometry and
  bearings, CSS/DPR viewport transforms, source-labelled geography schema and
  the fixture-only corridor loader. Local tag `map-contract-v1` identifies this
  contract checkpoint; moving it or publishing it requires explicit approval.
  At that checkpoint, calibration, cross-section seams and reviewed terrain
  remained future work; see the subsequent entry above.
