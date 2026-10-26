# Original terrain art library v1

`public/art/terrain-v1/manifest.json` describes **40 reusable images**, not one
image per hex: nine ground surfaces and 31 transparent overlays. Four original
Painter masters, generation provenance, SHA-256 hashes, crop rectangles and a
deterministic extraction script are included. No private scanned images were
provided to Painter or copied into these assets. Generated illustrations are
not evidence about historical geography, building plans or construction status.

## Ownership and delivery

The [map owner](https://ampcode.com/threads/T-01a171f6-55e2-718a-9d49-75285f7c2cc5)
confirmed the asset alignment contract on 2026-10-25. Kevin subsequently requested
live map integration and a usable default without private source artwork. The art
pack stays in `public/art/terrain-v1/`, with tooling in `scripts/terrain-art/`.
`crates/frontend/src/map/art.rs` consumes the manifest and typed observations;
small hooks in geography/research and map CSS expose it. Geometry, extraction and
source evidence remain unchanged. This work began from `origin/main`, not the
owner's unpublished map branch, and was then rebased onto the published full
survey in [PR #27](https://github.com/khallmark/campaign_for_north_africa/pull/27)
at [e5af0e7](https://github.com/khallmark/campaign_for_north_africa/commit/e5af0e7ed0a15c6c791c9654e258b76a1e796b27).
The expanded survey and shared-main rules were subsequently merged from
[fd497c8](https://github.com/khallmark/campaign_for_north_africa/commit/fd497c87883bd30d113f9c126edb1fc7b2e9ccd9),
preserving this renderer. That update changes transport evidence, not terrain
or geometry; the existing ground cache still verifies against its inputs.
The integration retains raster examinations, water-evidence tints and the owner's
terrain → links → railroad spine/ties → locations paint order.

## In the live map

**Original terrain** is enabled by default, even with no private tile manifest.
There is no missing-source-artwork banner. Source scan is an optional, initially
off layer, shown only when a valid local scan manifest exists. The two toggles
are independent: with both enabled, original ground is 55% opaque over the source
scan for aligned comparison. Either layer alone remains opaque. Turning both
off leaves the schematic geography mode usable without scans.

The renderer displays continuous, blended materials for typed reviewed/provisional
terrain observations and original stamps for typed points. Materials use global
board coordinates rather than restarting in each hex; transitions soften across
observed cells. Multiple materials use equal illustrative weights, not measured
area proportions or internal boundaries. **These fills are decorative presence
indicators, not claims of full-cell coverage.** Unknown cells remain gray;
fixture and unresolved values never select specific artwork. Hex evidence retains
the original provenance and review status. Try **B0526** for desert or **B5614**
for mountain using the Hex identity control.

Transport and rivers remain the map owner's **schematic lines**, not the artwork
strips: the model does not supply authored path centerlines. Explicit
`MapRecord::Hexside.depiction` hints now draw violet dashed unfinished-road or
unfinished-railroad strokes (rail adds separated ties), with inspector and legend
labels. All 38 source-backed hints remain unresolved and excluded from operational
`network_links`. Untagged ink remains unclassified; labels are never parsed to
choose a depiction. The transparent unfinished strips remain library-only until
authored paths exist. Minefield art also requires viewer-relative simulation state.

## Coverage is layered, not a terrain Cartesian product

| Layer | Artwork |
| --- | --- |
| Ground | Clear, Gravel, Salt Marsh, Heavy Vegetation, Rough, Mountain, Delta, Desert, Swamp |
| Transport/water strips | Road, Railroad, Track, Pipeline, Major River, Minor River; separate unfinished Road and Railroad |
| Side-feature strips | Ridge, Slope, Escarpment, Wadi, Coast |
| Domain/uncertainty stamps | Sea, neutral Unknown hatch |
| Locations | Major City, Village, Bir, Oasis, Well, Airfield, Landing Strip, Flying Boat Basin, Port |
| Effects | Three fortification levels, neutral Minefield, separate friendly/enemy badges, combined Village/Bir/Oasis legend stamp |

Manifest recipes cover all 16 `Terrain` values, 14 `HexsideFeature` values and
10 `PointFeature` kinds from `terrain.rs` / `map-v1.schema.json`. Up/down features
reuse the same artwork with opposite high-side orientation. The three fortification
levels have distinct stamps. Ports retain their `efficiency_ref`; the same port
illustration serves all chart rows without implying capacity.

`surface: null` means **preserve a separately evidenced surface, or show Unknown**.
It never means Clear. City, fortification and minefield effect rows must not erase
the underlying ground or invent it. The Village/Bir/Oasis combined stamp is a
legend symbol for the undifferentiated TEC row, not evidence that all three exist
at a location. Prefer a specific point stamp when the typed observation supplies it.
Minefield badges are viewer-relative simulation presentation, not permanent map
ownership. Keep the neutral minefield visible and place the badge separately,
with a textual friendly/enemy label; circle/diamond shapes distinguish the colors.

## Coordinates and placement contract

- The board is **14,310 × 4,632 board pixels**, with **7,180 section-local IDs**.
  `geometry::center`, `corners`, `edge` and `calibration.rs` remain authoritative.
  Never infer seam merges or replace these cells with a regular ideal hex grid.
- Delivery raster tiles being 512px does **not** establish per-hex art dimensions.
  Actual ground images are 402×402 texture swatches. Typical calibrated cell bounds
  are approximately 85.25×97.86 board pixels, but derive each bound from `corners`.
- The reusable source textures are not inherently seamless. The display baker
  overlaps deterministic rotated 256-board-pixel patches with cosine weights,
  then blends material masks with a 12-pixel radius. Texture coordinates are global,
  so neither hexes nor delivery tiles restart the pattern.
  Masks derive from actual cell corners; a final known-cell mask prevents the
  blur from classifying unknown neighbors. No section identities or seams are merged.
- Every asset declares its native `size`, `pivot`, `kind`, alpha mode and path
  relative to the pack root. Normalized coordinates have origin at top-left,
  +x right and +y down. Multiply normalized coordinates by native dimensions.
  `pivot: [0.5, 0.5]` is the canvas center, including transparent padding.
- Point stamps use `center(hex)` as their default attachment point. Start with a
  maximum stamp extent of 35–45 board pixels; badges 10–12 pixels. These are visual
  recommendations, not real-world dimensions. Keep labels readable and independently
  accessible. Do not rotate a building stamp to imply surveyed building orientation.
- Strips provide two measured visual `anchors`, near the left/right ends; native
  art extends beyond each anchor as a cap. They are **not connection evidence**.
  For a straight illustrative span, map the anchor pair to its requested endpoints,
  with the perpendicular scale controlling width. Start around 4–6 board pixels
  for road/rail, 3–4 for track/minor river, 7–10 for major river, 2–3 for pipeline.
  Preserve the major/minor distinction at any shared zoom.
- For actual hexside features, derive endpoints from `edge(side)`. `high_side_normal`
  is `[0,-1]` for Slope/Escarpment. Align that side toward the neighbor for Up,
  toward the canonical hex for Down. Use the manifest recipe, not enum names alone
  after reversing a crossing. Coast's `water_side_normal` is also `[0,-1]`; orient
  only with evidence of which side is water. Sea is a **domain stamp**, not a fill
  that turns a whole hex into sea.
- River/road/rail strips are independent RGBA decals. They can be rotated/scaled
  or later warped along an authored route, but they are **not seamless repeatable
  line textures**. Caps, bends, junctions, bridges and overlap handling require a
  renderer decision and evidence. The library does not invent these connections.

Suggested layer order: known ground or Unknown base → evidenced natural side
features → evidenced transport → location stamps → simulation effects → grid,
labels, review-state indicators and selection. This does not resolve the z-order
or legality of a road/river crossing; do not draw an unsupported bridge.

## Uncertainty and transportation safety

Missing/unresolved observations use the neutral `#d8dcdf` base and Unknown hatch,
plus their source label. **Blue barred marks near Sollum remain unresolved**, not
Railroad. Neither this pack nor its preview assigns them a transport sprite.

Review status (`reviewed`, `provisional`, `fixture`, `unresolved`) and construction
completion are separate concepts. A provisional road is not automatically an
unfinished road. `construction_variants` is a presentation extension, not a new
operational feature enum. The source-backed `TransportDepiction` contract supplies
explicit drawing evidence without resolving construction completion or movement.
If completion remains unknown, retain an uncertainty indicator instead
of choosing finished-looking artwork. A finished strip in the library does not
certify any real road or railroad as complete.

Current `network_links` are provisional canonical hexside crossings. They are
not geographic centerlines or legal movement. **Do not replace those schematic
connections with precise-looking art paths**. Keep the existing schematic mode
labeled as such until authored path geometry and evidence are available.
Non-detection is not absence, and a partial terrain patch does not establish
full-cell coverage. The blended display is explicitly illustrative; do not export
its material boundaries as geography or use them to determine movement. Inspect
the unchanged evidence records and compare the private source view for extent.

## Buffered raster display

`scripts/terrain-art/raster.py` uses the `map-model` export and original surface
assets only. It emits a six-level lossless WebP pyramid in `ground/`, separate
from private source tiles. The 382 tiles are bundled; no runtime image generation
or private scan is required. The manifest fingerprints geometry, typed material
assignments, original surface hashes and generator, and records every tile hash.

Original ground loads at most 100 detail images plus one overview; enabling
Source scan also loads its separate bounded tile ring. Ground and SVG
evidence keep a quarter-viewport buffer on each side. Both SVG drawing surfaces
are bounded to that buffered viewport, not the full board. Panning translates those layers;
leaving the buffer, zooming, resizing or changing evidence controls refreshes them.
Unknown gray, labels, selection, route ordering and hit testing remain independent
of the raster. Source scan retains its own private tile loader and the same camera;
hidden source detail tiles are removed rather than downloaded in the background.
Disabling original terrain also releases its ground nodes; enabling the source
layer does not disable or release the original terrain layer.

## Rebuild and verify

From the repository root, with `uv` and the existing Rust toolchain:

```sh
uv run scripts/terrain-art/build.py --write
uv run scripts/terrain-art/build.py --check
uv run scripts/terrain-art/raster.py --write
uv run scripts/terrain-art/raster.py --check
uv run --with Pillow==12.0.0 --with numpy==2.3.4 python -m unittest discover -s scripts/terrain-art
cargo run --quiet --locked -p cna-map --bin map-model > /tmp/terrain-art-geometry.json
uv run scripts/terrain-art/preview.py \
  --geometry /tmp/terrain-art-geometry.json --output .amp/in/artifacts
```

The scripts pin Pillow 12.0.0 without changing shared dependency files. Preview
text uses the installed DejaVu Sans font. `--check` is read-only: it verifies
schema vocabulary coverage, master hashes, byte-for-byte regenerated images and
manifest, genuine transparency, clear padding and visible interior construction
gaps. Regenerating Painter art itself is nondeterministic; extract from the
checked-in masters to reproduce these exact deliverables.

The preview uses the exported **C3617 outline only**, translated to sample cards;
none of the sample materials/features are assigned to C3617. It shows nine
surfaces and six composed examples, including finished versus unfinished
transport and neutral Unknown. A second contact sheet composites **all 31 RGBA
overlays on both cream and dark slate** for halo/background inspection.
These are offline artwork previews, not screenshots or proof of renderer integration.

Before integrating linear-feature art, test high-side orientation in both traversal
directions, preserve review-status labels, and verify shared-side uniqueness.
Coordinate future map renderer/CSS and map-test changes with the map owner.
No private scan is needed to serve the original terrain mode.
