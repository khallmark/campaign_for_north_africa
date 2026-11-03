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
| Malta | 58 | 18 / 16 / 17 / 7 | 18 |
| A | 1,000 | 866 / 62 / 64 / 8 | 708 |
| B | 1,961 | 1,885 / 40 / 34 / 2 | 1,386 |
| C | 1,520 | 1,438 / 41 / 36 / 5 | 859 |
| D | 1,240 | 1,166 / 37 / 36 / 1 | 784 |
| E | 1,297 | 979 / 272 / 41 / 5 | 948 |

The current combined model has **271 surveys / 23,983 observations**: 4,851 terrain,
18,984 hexside and 148 point observations. **7,752 are provisional, 16,231 unresolved,
none human-reviewed.** Terrain observations cover 4,703 cells; 2,373 have none.
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
Separately, 119 unresolved observations carry explicit unfinished construction
depictions: 80 road observations (70 distinct sides) and 39 railroad observations.
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
| C | 32 | 0 | 425 |
| D | 36 | 26 | 292 |
| E | 171 | 167 | 151 |
| Malta | 0 | 0 | 0 |

Zero means no typed crossings, not confirmed absence. In particular, disconnected
crosses on unfinished lines in C must not become completed railroads. Blue barred
hexside marks remain unclassified. The renderer paints railroad spines and ties
above roads, so coincident schematic links remain visible when the scan is hidden.

All 119 retained blue-dot candidates were visually inspected for labels by an
agent. The combined Village/Bir glyph does not establish a water-source category;
labels remain unresolved observations, including uncertain spellings and an
unlabelled dot near Rifi. Additional source inspection records Malta's four named
airfields and two flying-boat basins, plus ten Major City glyph cells across
Benghazi, Tobruk, Bardia, Alexandria and Cairo. Two detected oasis glyphs and two
port glyphs do not establish complete recall. Other location symbols, labels,
fortifications and operational port associations remain incomplete. No capacity,
city level, movement cost, water amount, bridge rule or legal movement is inferred.

### Source-located symbols and reported track omissions

The compact model's `examinations[hex].points` retains 123 detected glyphs
(119 Village/Bir dots, two oasis and two port symbols) at their measured board-pixel
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

The screenshot-led C1832/Qara audit checks 46 consecutive crossings across five
regional routes, including the adjoining eastern and northeastern branches. Six
omissions have source-cited corrections: C1831 east, C1832 northeast, D1008
northeast, D1208 northwest, D1809 northeast and D1915 northeast. The C crossings
are corner-sensitive; Qara's dot/wadi, the escarpment fringe, curved dashes and
converging branches interrupt the detector's narrow straight-stroke evidence.
These corrections preserve its raw uncertainty rather than relaxing safeguards
globally. Tests also reject a false detour through C1932 and the nonexistent
D1114 east shortcut: that route passes through D1014. This regional audit does
not establish complete map-wide recall or exact centerline geometry.

The Barrani–Matruh coastal audit checks 117 within-section crossings: 80 Track,
19 Road and 18 Unfinished Railroad. Nine missing track crossings receive
source-cited provisional observations, covering the Barrani–Bir Khamsa route,
Piccadilly's parallel track/unfinished-rail corridor, Qatrani and Mingar Qaim.
Twelve missing unfinished-railroad sides extend the Charing Cross sample west
through D3711/D3710 and D3609 to D3601. These disconnected crosses remain
unresolved depictions, never completed railroad links. All 19 checked road
crossings were already present. Tests reject a false road detour through D3610
and an invented southward extension from Bir Talab's eastern track into D2916.
The C/D seam still has no cross-section links, so this does not resolve the
visible section-boundary breaks. Escarpment rendering and exact source-shaped
route geometry are separate work; the detector's raw findings remain unchanged.

The Malta whole-cell audit adds four provisional ground observations. The A audit
adds 84 Clear occurrences, nine partial Mountain cores retaining existing Rough
fringes, and one partial Gravel occurrence. Of 2,373 cells without typed ground,
source-surface measurements report 1,941 land, 228
water, 180 mixed and 24 unresolved. A land
measurement or a place label does not establish Clear terrain. A neutral unknown
texture is presentation, not a terrain observation.

## Whole-cell visual audit is recorded separately

`data/map/source-reviews.json` contains explicit agent-authored inspection batches.
`data/map/source-review-ledger.json` expands them to **every inventory cell**, with
source crop, five family findings, survey references and uncertainty. Missing
batches produce `not_reviewed`, even when the detector found a feature or a prior
spot survey exists. `agent_reviewed` means the full outlined cell was visually
inspected, **not** that every interpretation was resolved or human-approved.
`observed`, `not_observed` and `unresolved` describe family findings; source-symbol
absence does not certify historical absence. Every record keeps `human_reviewed:
false`. Inventory membership does not establish playability.

The audit starts at integration commit `add8fb4a8f4aec0079f1eb5b916a6d58f35a852c`.
Recorded coverage is **58/58 Malta and 656/1,000 A cells; 6,362 cells remain
not_reviewed**, including all of B–E and A2101 onward. Malta source crops and corners match that pin exactly. Full-inset
context and each individual outlined crop were inspected at 2× against TEC. Four
airfields and two flying-boat basins match existing surveys; no road, rail, track,
unfinished transportation, Village/Bir dot or Major City cluster was observed.
Three inland airfield cells (0604, 0704, 0802) have affirmative plain land hidden
from the blank-pixel test by glyphs/text. A small western Rough patch enters 1002.
Both findings have separate provisional correction surveys. Coastal 0505, 0804,
1301 and other mixed cells remain untyped; Rough patches do not imply homogeneous
terrain. Water, coast sides, capacities and operational interpretations remain
explicitly unresolved rather than inferred from the inset's labels or numerals.

The two southern A batches cover every cell A0101–A1033. Individual outlined
crops were inspected at 2×, with enlarged edge/crossing controls. They add 25
affirmative Clear occurrences beneath tracks/labels or beside nearby patterns,
five dark Mountain cores without erasing existing Rough, and two missing Track
crossings: **A0430 E → A0431** and **A0703 NW → A0802**. Maradah's point remains
operationally unresolved; neighboring lettering is not another settlement.
Wadi meanders and stippled slope/ridge-like side bands retain directional
uncertainty, and A0925 remains untyped rather than treating its narrow side band
as a Mountain core. A0433's source Track continues toward B0401, but the audit
does not invent cross-section adjacency: geometry/seam adjudication is separate.

The next batch covers all 165 cells A1101–A1533, adding thirteen Clear occurrences,
four partial Mountain cores, partial Gravel inside A1518 and the missing Track
**A1523 E → A1524**. The separate A1523 NE branch remains intact. Enlarged controls
resolve the Track over Wadi in A1503 and distinguish A1518's interior Gravel rings
from near-grid fragments in A1418/A1325, which remain untyped. Maaten Groter and
Maatan Belcleibat retain unresolved operational categories; no new location is
created from neighboring lettering. Stippled side bands remain distinct from
Mountain cores, with side type/direction explicitly unresolved.

The A1601–A2033 batch covers 161 retained cells, adding 46 affirmative Clear
occurrences and three Tracks: **A2003 NE → A2104**, **A2032 NE → A2133** and
**A2032 E → A2033**. The separate A1933–A2033 branch remains. A2033's southeastern
continuation leaves the section-local inventory and is a geometry-owner finding,
not a new inferred seam. Near-grid Gravel, coastal slivers and stippled side bands
retain explicit uncertainty; existing Clear at A1924/A1928 is not a claim that
their marginal Gravel fragments are absent.

The same audit found **Bir es Suera (A1622)** missing from the point detector:
antialiased edge loss left 118 exact-blue pixels in a 13×13 box, just below the
70% fill gate. A two-percentage-point margin also recovers **Qotifya (D3227)** and
**Galal (D3326)**. A whole-inventory delta check found only those three new dots;
each dot and label was inspected at 4× before acceptance. Synthetic controls keep
hollow letters and neighboring-cell dots excluded. Measured centers and labels
retain unresolved Village/Bir categories. Individual `review_dates` preserve the
original 116 label-review dates. The D-cell spot checks and paired A2104/A2133
crossing checks do **not** count as whole-cell visual coverage.

```sh
python3 scripts/build-map-review-ledger.py --check
# Only after inspecting source and authoring explicit cell findings:
python3 scripts/build-map-review-ledger.py --write
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.test_map_review -v
```

The builder verifies original source hashes and exact geometry, rejects duplicate
or nonexistent identities, incomplete family findings and inapplicable survey
references. It does not review pixels or change model classifications. Source
images/crops remain private; the ledger contains references and notes only, and is
not mounted into the renderer. Tests included in `npm check` check the committed
ledger against current model geometry without needing copyrighted raster files.
Geometry/source changes require explicit reconciliation, not blind regeneration.

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
