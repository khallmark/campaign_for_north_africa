# Regenerable source-grid research model

The model enumerates section-local lattice centers inside calibrated VASSAL
section polygons, excluding row zero: **7,076 identities**, with six board-pixel
corners each and **20,375 canonical internal shared sides**. Counts are Malta 58,
A 1,000, B 1,961, C 1,520, D 1,240 and E 1,297. This is a complete enumeration
under the **center-in-polygon, positive-row policy**, not a complete terrain or playable-map
transcription. Edge outlines are full Voronoi cells, not clipped coastline shapes.
Cells whose centers fall outside the polygon are excluded even if a corner enters
it. Raster-edge cells can extend outside the image. Sea cells are included.

Geometry reuses [calibration](calibration.md) and [identities](hex-identities.md).
The module-derived row-zero labels do not appear on the original map. Removing
those 104 cells does not renumber or reposition any retained cell: A3033 remains,
while B3000 is excluded. The inventory contains no cross-section edges.
Neither parsing nor polygon membership confers
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
.venv-map/bin/python -m unittest scripts.test_map_extraction -v
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

Transport candidates are gated after antialias recovery, not solely on exact
palette pixels: otherwise a visible thin track can disappear before classification.
Dash fits require observed ink beyond the grid band on both sides within the
finite hexside span, so a chord across a bend cannot invent a corner crossing.
Repeated hollow road capsules produce `unfinished_road` depictions with unresolved
status, never completed-road network links. Three distinct aligned capsules are
required; multiple holes in one grid-contaminated glyph do not count as repetition.
Synthetic controls distinguish those capsules from solid dashes, a single capsule,
and continuous completed-road lines.

The browser uses the same Rust geometry and survey inputs as SQLite; it does not
download SQLite. Search accepts only inventory members. Inspect mode is separate
from counter placement; click inspects the pointer cell and Enter inspects the
viewport center. Dragging still pans. Grid outlines appear at zoom ≥0.22, labels
at ≥0.65. Source scan can be hidden to inspect the independent vector model.
Overlapping section identities are separately searchable; pointer hit testing
retains the existing section priority and does not resolve seam identity.

## Whole-map raster examination is not complete semantic transcription

All **7,076 cells and 20,375 internal sides** have been examined by the detector,
with **35,380 cell/family ledger entries**. The five families are surface, terrain,
transport, hexside and locations. `examination_coverage` reports their states by
section. `detected` means a provisional interpretation, `candidate` means ink
needs interpretation, `not_detected` means this detector found none (not confirmed
absence), and `unresolved` means no supported interpretation. Unsupported symbols
still need examination by another method. No family is marked complete and no
automated or agent-visual record is human-reviewed.

| Section | Examined cells | Land / mixed / water / unresolved | Cells with terrain observations |
| --- | ---: | --- | ---: |
| Malta | 58 | 18 / 16 / 17 / 7 | 14 |
| A | 1,000 | 866 / 62 / 64 / 8 | 623 |
| B | 1,961 | 1,885 / 40 / 34 / 2 | 1,386 |
| C | 1,520 | 1,438 / 41 / 36 / 5 | 859 |
| D | 1,240 | 1,166 / 37 / 36 / 1 | 784 |
| E | 1,297 | 979 / 272 / 41 / 5 | 948 |

The current combined model has **251 surveys / 23,849 observations**: 4,753 terrain,
18,951 hexside and 145 point observations. **7,633 are provisional, 16,216 unresolved,
none human-reviewed.** Terrain observations cover 4,614 cells; 2,462 have none.
Water is not automatically Sea: inland water shares its ink. Mixed water/land is
not automatically a Coast side. Clear requires an affirmative near-blank source
swatch; all failed or unsupported classifications remain unknown.

The extracted terrain occurrences are Clear 2,025, Rough 765, Desert 694,
Gravel 447, Delta 326, Salt Marsh 291, Mountain 101, Swamp 26 and Heavy Vegetation 8.
Broad mountain cores exclude narrow slope bands; partial patches remain evidence,
not an assignment that an entire hex has one uniform surface. Original manual
surveys remain separate and may overlap extracted observations.

Clear requires at least 99.5% matching blank background in the inset polygon and
fewer than five terrain/water-ink pixels in the full polygon. Transport/grid
antialiasing is excluded from that terrain test; failed matches stay unresolved.

Transportation extraction currently yields 370 Road, 205 Railroad and 1,538 Track
crossings; 705 Wadi side candidates are typed provisionally. Gray grid occlusion
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
An additional visual survey corrects five grid-contaminated track crossings near
Fort Maddelenna and Bir el'esc Scegga (C3018 E, C3020 E, C2717 NE, C2820 NE,
C3417 E). All seven gray-dashed route traces in source crop (6950,2010,770,760)
are checked crossing by crossing, including branches; endpoint reachability alone
had missed these gaps. A wider-context dash classifier now recovers those same
five crossings and 227 other previously untyped track findings across A–E. It
fits narrow dashes on both sides of the grid and requires three source-ink runs
with short gaps; it does not join disconnected graph endpoints. All 232 additions
were agent-inspected in source-side contact sheets, not human-reviewed. Existing
pixel counts and narrow-strip measurements are retained alongside new context
evidence. The sharp bend at C3008 E and two Sidi Azeiz track crossings beside
unfinished railroad symbols (C4217 E, C4118 NE) are supplied by source-cited manual
observations instead. The C2908–C3008–C3009 and Sidi Azeiz west/east and southwest/northeast
routes have consecutive-crossing tests.
No track is invented across C3019–C3020's blue barred side. At B4902 E, nearby
continuous rail and dashed track both trigger; the mixed detector finding now
stays unresolved instead of selecting Railroad. Its raw measurements remain.
Conservative detection still leaves routes fragmented and uncertain.
**18,903 side ink findings remain untyped**, with individual crops and measurements;
15,989 extracted unresolved side observations expose generic candidates. Recovering
antialiased ink admits more candidates, including ambiguous grid/text strokes;
these are evidence to inspect, not confirmed routes or operational links.
Separately, 107 unresolved observations carry explicit unfinished construction
depictions: 80 road observations (70 distinct sides) and 27 railroad observations.
The 69 retained automatically recognized road sides include all 38 checked Siwa–Matruh
crossings and source-backed stretches near Martuba, El Adem and Sollum. All 69
were agent-inspected against enlarged source crops; ten overlap existing manual
observations. They are excluded from network links, not counted as completed
Road/Railroad, and do not establish scenario completion. The construction renderer
displays these observations separately from completed transportation links.
Slopes/escarpments still need directional interpretation; most river/coast sides
need classification. There are no cross-section links or assumed operational
junctions. Network endpoints are transcription limits, not real termini.

Combined distinct typed crossings (manual plus extracted; duplicates removed):

| Section | Road | Railroad | Track |
| --- | ---: | ---: | ---: |
| A | 76 | 7 | 248 |
| B | 57 | 5 | 448 |
| C | 32 | 0 | 419 |
| D | 36 | 26 | 283 |
| E | 171 | 167 | 151 |
| Malta | 0 | 0 | 0 |

Zero means no typed crossings, not confirmed absence. In particular, disconnected
crosses on unfinished lines in C must not become completed railroads. Blue barred
hexside marks remain unclassified. The renderer paints railroad spines and ties
above roads, so coincident schematic links remain visible when the scan is hidden.

All 116 retained blue-dot candidates were visually inspected for labels by an
agent. The combined Village/Bir glyph does not establish a water-source category;
labels remain unresolved observations, including uncertain spellings and an
unlabelled dot near Rifi. Additional source inspection records Malta's four named
airfields and two flying-boat basins, plus ten Major City glyph cells across
Benghazi, Tobruk, Bardia, Alexandria and Cairo. Two detected oasis glyphs and two
port glyphs do not establish complete recall. Other location symbols, labels,
fortifications and operational port associations remain incomplete. No capacity,
city level, movement cost, water amount, bridge rule or legal movement is inferred.

### Source-located symbols and reported track omissions

The compact model's `examinations[hex].points` retains 120 detected glyphs
(116 Village/Bir dots, two oasis and two port symbols) at their measured board-pixel
centers. `symbol` is `village_bir_dot`, `oasis` or `port_icon`; `label` is optional,
`status` is `unresolved` or `provisional`, and `source_refs` identifies the source
crop and legend under the model's pinned SHA256 values. For example, C3111 carries
El Cuasc at `[6567.0, 2324.1]`, with `symbol: village_bir_dot` and
`status: unresolved`. A renderer can show its source symbol and name without
promoting the operational observation to a town, water supply or port. Positions
are validated against the owning hex's polygon, not global hit-picking across
unresolved seams. Raw template scores and label crops remain in the ledger.
Antelat remains at A3233; its duplicate B3200 association is excluded with row zero.

The October 28 comparison of El Cuasc, the track north of El Grein and Ain Melfa
found four omitted track crossings across six inspected route sequences. The
three source-cited correction surveys retain provisional status and leave raw
detector evidence unchanged. C3011 east and C1119 northwest fall in the detector's
12% endpoint exclusion zone; C2116 east has a sharp turn that fails its straight
dash corridor. These are conservative-detector false negatives, not missing scan
pixels. The correction does not weaken those checks globally or assert complete
track recall. Regression tests check each consecutive crossing rather than mere
reachability through an alternate route. Enlarged source crops against calibrated
side endpoints also rule out four initially misread crossings: the El Cuasc route
uses C3111 → C3011 → C3012, not C3112, and the northern bend uses C2318, not C2319.
Tests reject those unsupported detours. Blue barred sides remain unclassified.

The later C2410 east and C1415 northwest reports exposed an exact-palette gate
that discarded visible antialiased tracks before dash classification. Moving the
gate after recovery adds 12 source-inspected automatic crossings. Requiring actual
ink on both sides of the finite edge removes the false C0925 east chord. Two
source-cited corner corrections add C2414 east and C0825 northwest (to C0925).
The Siwa–Matruh hollow-capsule route is tested across every internal crossing:
C0127 through C0426 to C1833, then D1901 to D3612. D1800 is excluded with row zero;
no replacement C1833/D1901 seam link or
scenario completion is inferred. C0436 is outside the calibrated inventory;
the source route reaches D3612, not D0612.

B1507 has two further source-cited corner corrections: its northwest crossing
from B1606 and its east crossing into B1508. Tests cover B1706–B1606–B1507–B1508–B1509
and reject a detour through B1607. Raw detector findings and its conservative
endpoint exclusion remain unchanged.

No retained ground assignments changed. Of 2,462 cells without typed ground, source-surface
measurements report 2,030 land, 228 water, 180 mixed and 24 unresolved. A land
measurement or a place label does not establish Clear terrain. A neutral unknown
texture is presentation, not a terrain observation.

## SQLite contracts for later simulation research

`hexes` contains section identity, center, JSON outline and terrain completeness.
`hexsides` contains canonical endpoints and JSON edge geometry. `observations`
joins surveys and provenance, with normalized `observation_sources` pointing to
hash-pinned `sources`. `coverage` reports per-section geometry/terrain counts.
Schema version 2 added `cell_measurements`, `side_examinations`, `examinations` and
the `examination_coverage` view. These are detector coverage, not review approval.
Schema version 3 adds nullable `observations.depiction`, restricted to unresolved
hexside observations; see the [depiction contract](terrain-schema.md#unfinished-transportation-depiction).
Schema version 4 adds the `source_points` view over the existing immutable cell
evidence, exposing hex, point index, symbol, center, optional label, status and
source references. It does not duplicate observations or feed `network_links`.

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
