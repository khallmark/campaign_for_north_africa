# Regenerable source-grid research model

The model now enumerates every representable section-local lattice center inside
its calibrated VASSAL section polygon: **7,180 identities**, with six board-pixel
corners each and **20,684 canonical internal shared sides**. Counts are Malta 67,
A 1,013, B 1,989, C 1,545, D 1,253 and E 1,313. This is a complete enumeration
under the **center-in-polygon policy**, not a complete terrain or playable-map
transcription. Edge outlines are full Voronoi cells, not clipped coastline shapes.
Cells whose centers fall outside the polygon are excluded even if a corner enters
it. Raster-edge cells can extend outside the image. Sea cells are included.

Geometry reuses [calibration](calibration.md) and [identities](hex-identities.md).
The inventory preserves overlapping identities (for example A3033/B3000) and
contains no cross-section edges. Neither parsing nor polygon membership confers
playability. Original printed-number equivalence and seams remain unresolved.
Calibration's 30 measured anchors constrain sampled alignment, not every cell.

## Regeneration and inspection

Hydrate the existing original sources using the repository setup first. Python
3.10+ with SQLite 3.37+ and the pinned Rust toolchain are required. No extra Python
package is required for the SQLite importer. Re-examining the raster additionally
requires Python 3.11+ and the packages pinned in `requirements-map.txt`:

```sh
python3 -m venv .venv-map
.venv-map/bin/pip install -r requirements-map.txt
.venv-map/bin/python scripts/extract-map.py --check
# After reviewing changed rules, labels or sources:
.venv-map/bin/python scripts/extract-map.py --write
python3 scripts/import-map-db.py --write
python3 scripts/import-map-db.py --check
python3 -m unittest discover -s scripts -p test_map_db.py -v
cargo test --locked -p cna-map
```

The ignored `data/map/map.sqlite` is disposable derived data, not an editing
surface. `--check` (also the default) compares the entire logical database without
writing. `--write` atomically replaces an existing database only when its generator
marker belongs to this importer. `--output` accepts another private `.sqlite`
path, never `public/` or `dist/`. Do not write while another process has this
database open for mutation.

The Rust `map-model` binary exports geometry and expands `data/map/surveys.json`
and `data/map/extracted-surveys.json` into the existing `MapRecord` vocabulary.
`--geometry-only` exports geometry without validating old extraction, allowing
regeneration after a geometry correction. The importer verifies the original
map, TEC and module XML SHA256 values, and stores fingerprints of the census,
calibration, schema, exporter, vocabulary, surveys and importer. A changed source
requires review, not a blind hash update. Correct geography in surveys, retaining
source crop references, review status, reviewer, date and explanation, then
regenerate. Keep conflicting interpretations unresolved until adjudicated.

`scripts/extract-map.py` examines all source polygons and internal side strips
using the pinned palette and templates in `data/map/extraction-rules.json`.
`data/map/examination.json` retains each cell/side's crop, pixel measurements,
candidate symbols and uncertainty; `data/map/location-labels.json` retains agent
visual label associations. The compact generated surveys and browser ledger are
fingerprinted together. SQLite import rejects stale generator/rule/geometry pins
and disagreement with that compact ledger. No raster pixels are embedded in the
database or generated JSON. Regeneration uses the private original PNG and TEC.

The browser uses the same Rust geometry and survey inputs as SQLite; it does not
download SQLite. Search accepts only inventory members. Inspect mode is separate
from counter placement; click inspects the pointer cell and Enter inspects the
viewport center. Dragging still pans. Grid outlines appear at zoom ≥0.22, labels
at ≥0.65. Source scan can be hidden to inspect the independent vector model.
Overlapping section identities are separately searchable; pointer hit testing
retains the existing section priority and does not resolve seam identity.

## Whole-map raster examination is not complete semantic transcription

All **7,180 cells and 20,684 internal sides** have been examined by the detector,
with **35,900 cell/family ledger entries**. The five families are surface, terrain,
transport, hexside and locations. `examination_coverage` reports their states by
section. `detected` means a provisional interpretation, `candidate` means ink
needs interpretation, `not_detected` means this detector found none (not confirmed
absence), and `unresolved` means no supported interpretation. Unsupported symbols
still need examination by another method. No family is marked complete and no
automated or agent-visual record is human-reviewed.

| Section | Examined cells | Land / mixed / water / unresolved | Cells with terrain observations |
| --- | ---: | --- | ---: |
| Malta | 67 | 18 / 21 / 21 / 7 | 15 |
| A | 1,013 | 879 / 62 / 64 / 8 | 628 |
| B | 1,989 | 1,912 / 40 / 35 / 2 | 1,402 |
| C | 1,545 | 1,462 / 42 / 36 / 5 | 865 |
| D | 1,253 | 1,179 / 37 / 36 / 1 | 786 |
| E | 1,313 | 994 / 273 / 41 / 5 | 956 |

The current combined model has **237 surveys / 13,695 observations**: 4,792 terrain,
8,757 hexside and 146 point observations. **7,446 are provisional, 6,249 unresolved,
none human-reviewed.** Terrain observations cover 4,652 cells; 2,528 have none.
Water is not automatically Sea: inland water shares its ink. Mixed water/land is
not automatically a Coast side. Clear requires an affirmative near-blank source
swatch; all failed or unsupported classifications remain unknown.

The extracted terrain occurrences are Clear 2,030, Rough 775, Desert 708,
Gravel 453, Delta 326, Salt Marsh 294, Mountain 102, Swamp 26 and Heavy Vegetation 8.
Broad mountain cores exclude narrow slope bands; partial patches remain evidence,
not an assignment that an entire hex has one uniform surface. Original manual
surveys remain separate and may overlap extracted observations.

Clear requires at least 99.5% matching blank background in the inset polygon and
fewer than five terrain/water-ink pixels in the full polygon. Transport/grid
antialiasing is excluded from that terrain test; failed matches stay unresolved.

Transportation extraction currently yields 370 Road, 206 Railroad and 1,325 Track
crossings; 707 Wadi side candidates are typed provisionally. Gray grid occlusion
may be bridged, but light gaps between unfinished dashes/crosses may not. Known
terrain swatches take precedence over interpolation that resembles road ink.
Gray route ink is fitted over the terrain palette; connected stroke length is
measured along the route, so shallow crossings and termini are not rejected just
for lacking a long normal-axis span. Solid escarpment fringes are excluded from
gray route evidence; isolated antialiased rail pixels are not escarpment patches.
Source controls cover finished roads/railroads, unfinished construction and a
false railroad candidate at D2224 NE. Separate visual surveys recover the Bardia
and Sollum road branches obscured by blue symbols. Connectivity tests cover the
Bardia–Sollum road and Fort Maddelenna–Bir el Sheferzen track examples.
Conservative detection leaves many obvious routes fragmented and uncertain.
**9,019 side ink findings remain untyped**, with individual crops and measurements;
6,128 unresolved side observations expose the relevant candidates in the overlay.
Slopes/escarpments still need directional interpretation; most river/coast sides
need classification. There are no cross-section links or assumed operational
junctions. Network endpoints are transcription limits, not real termini.

Combined distinct typed crossings (manual plus extracted; duplicates removed):

| Section | Road | Railroad | Track |
| --- | ---: | ---: | ---: |
| A | 76 | 7 | 220 |
| B | 57 | 6 | 399 |
| C | 32 | 0 | 337 |
| D | 36 | 26 | 236 |
| E | 171 | 167 | 133 |
| Malta | 0 | 0 | 0 |

Zero means no typed crossings, not confirmed absence. In particular, disconnected
crosses on unfinished lines in C must not become completed railroads. Blue barred
hexside marks remain unclassified. The renderer paints railroad spines and ties
above roads, so coincident schematic links remain visible when the scan is hidden.

All 117 detected blue-dot candidates were visually inspected for labels by an
agent. The combined Village/Bir glyph does not establish a water-source category;
labels remain unresolved observations, including uncertain spellings and an
unlabelled dot near Rifi. Additional source inspection records Malta's four named
airfields and two flying-boat basins, plus ten Major City glyph cells across
Benghazi, Tobruk, Bardia, Alexandria and Cairo. Two detected oasis glyphs and two
port glyphs do not establish complete recall. Other location symbols, labels,
fortifications and operational port associations remain incomplete. No capacity,
city level, movement cost, water amount, bridge rule or legal movement is inferred.

## SQLite contracts for later simulation research

`hexes` contains section identity, center, JSON outline and terrain completeness.
`hexsides` contains canonical endpoints and JSON edge geometry. `observations`
joins surveys and provenance, with normalized `observation_sources` pointing to
hash-pinned `sources`. `coverage` reports per-section geometry/terrain counts.
Schema version 2 adds `cell_measurements`, `side_examinations`, `examinations` and
the `examination_coverage` view. These are detector coverage, not review approval.

`network_links` exposes both directions of each road/railroad/track/pipeline
crossing, including its observation ID, survey and review status. It supports
connectivity research, **not legal movement**. It is not a geometric centerline
or a claim that every pair of routes in a hex forms an operational junction.
Consumers must retain type, evidence status and incomplete endpoints, and need
separate reviewed movement/transfer rules. The overlay draws these center-to-center
links schematically; other hexside features follow the actual shared edge.

```sql
SELECT * FROM coverage;
SELECT * FROM examination_coverage ORDER BY section,family,status;
SELECT hex, evidence FROM cell_measurements WHERE surface='unresolved';
SELECT hex, direction, evidence FROM side_examinations WHERE hex='D3512';
SELECT from_hex, to_hex, feature, status
FROM network_links WHERE from_hex='D3714';
SELECT o.kind, o.status, o.value, s.notes, os.reference, src.sha256
FROM observations o JOIN surveys s ON s.id=o.survey
JOIN observation_sources os ON os.observation=o.id
JOIN sources src ON src.path=os.source
WHERE o.hex='D3714';
```

## Private review remains opt-in

Original artwork and generated tiles retain the [distribution gate](licensing.md).
SQLite files and their sidecars are denied by Vite even when private tile review
is enabled. Keep the database outside public/distribution directories; its
presence does not license source publication. `tests/private-review.test.mjs`
covers catalog and map database requests with and without the review opt-in.
Production builds continue to run the source distribution guards.
