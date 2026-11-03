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
| A | 1,000 | 866 / 62 / 64 / 8 | 828 |
| B | 1,961 | 1,885 / 40 / 34 / 2 | 1,832 |
| C | 1,520 | 1,438 / 41 / 36 / 5 | 859 |
| D | 1,240 | 1,166 / 37 / 36 / 1 | 784 |
| E | 1,297 | 979 / 272 / 41 / 5 | 948 |

The current combined model has **325 surveys / 24,660 observations**: 5,423 terrain,
19,076 hexside and 161 point observations. **8,416 are provisional, 16,244 unresolved,
none human-reviewed.** Terrain observations cover 5,269 cells; 1,807 have none.
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
adds 199 Clear occurrences, nine partial Mountain cores retaining existing Rough
fringes, five partial Gravel occurrences and one partial Rough occurrence.
The twelve B batches add 443 Clear occurrences, three partial Rough
occurrences and six partial Mountain cores, retaining existing Rough where present.
Of 1,807 cells without typed ground, source-surface measurements report 1,376 land, 228
water, 179 mixed and 24 unresolved. A land
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
Recorded coverage is **58/58 Malta, 1,000/1,000 A and 1,961/1,961 B cells; 4,057 cells
remain not_reviewed**, covering all of C–E. Rosetta's anchor is a point-only spot
check and does not count as whole-cell review. This is agent source review, not complete interpretation
or human approval. Malta source crops and corners match that pin exactly. Full-inset
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

The A2101–A2633 batch reconciles another 120 retained cells. It adds 42 Clear
occurrences, partial Gravel at A2431/A2631, and two missing Tracks beneath blue
dots: **A2331 E → Belindah A2332** and **A2530 NW → Agadabia A2629**. The separate
Agadabia eastern branch remains. A2429's northern dashed stub ends beneath side-band
ink; no connection to Agadabia is inferred. Marble Arch's road outer stroke clips
the A2208/A2209/A2109 vertex: retain the existing footprint observation but keep
centerline/branch interpretation unresolved. A2133/A2633 eastern continuations
are seam-owner findings, not new adjacency. Belindah's near-grid Gravel remains
untyped, and existing Clear at A2531 retains marginal-Gravel uncertainty.

The A2701–A3333 batch reconciles 60 more retained cells, adding 23 Clear
occurrences, partial Gravel at A2732/A2733 and **A2703 NW → A2802 Road**. The
short closed Road tip is visible beyond the shared side; no farther continuation
is inferred. At the coastal A2929/A3028/A3029 vertex the Road center stays east
of A3028, so no extra crossing is added. Antelat's A3033/A3233 eastern continuations
are seam-owner findings, not detours through A3133.

Three additional source-anchor templates recover **Nofilia (A2703), Zuetina
(A2928) and Benghazi (A4827)** at measured centers without lowering the 0.88
match threshold. The full-source candidate search found five anchors, including
the unchanged Matruh and Ras el Ali symbols; this does not certify complete
port recall. Nofilia's glyph straddles A2803 but its measured center lies about
two pixels inside A2703. Footprint membership, settlement association, port
capacity and efficiency remain unresolved. Benghazi's point-only check earns
no whole-cell review credit. Templates preserve distinct raster/grid-overlap
variants rather than relaxing the match threshold for all white patterns.

The A3427–A4033 batch reconciles 56 more cells, with 24 affirmative Clear
occurrences and five restored Tracks: **A3727 NE → A3827, A3929 NW → A4028,
A4030 NW → A4130, A4031 NW → A4131, A3933 NW → A4032**. Paired source controls
distinguish the short branches and dot/scarp overlaps from Railroad ties.
Carcura's Track bypasses A3828 through A3827 → A3928; no extra crossing is added
there. The eastern Tracks at A3433/A3633/A3833/A3933 remain unresolved section
continuations. Broad bands stay untyped, Wadi/scarp-tip membership at A3833
remains uncertain, and no Up/Down, passability or operational junction is inferred.

The remaining 108 northern A cells add 26 Clear occurrences, partial Rough at
A4933, seven Railroad crossings, two Road crossings and ten rail-parallel Tracks
around Giardina, Benghazi and Er Regima. Four more Tracks restore the eastward
Ghemines, Soluch, A4132 and A4431 dot/scarp-obscured crossings.
Benghazi's three Training Area figures
are recorded as unresolved point evidence; they are not another city, port or
capacity assignment. Benina's olive stipple stays an unresolved side band, not
Rough or Salt Marsh. A4832 stays untyped at its uncertain northwestern fringe.

**A4832 northwest Railroad remains disputed.** Its transverse tie approaches
A4932's bottom vertex while the visible centerline runs from A4831 east through
A4832 northeast to A4933. The prior provisional observation is preserved with
an explicit unresolved review, not silently removed or treated as another branch.
Eastern Track/Railroad continuations at A4933, the hooked Track at A5033 and
Road at A5533 remain seam-owner findings. Source review does not authorize
changes to canonical geometry or operational routes.

The first 165 southern B cells add affirmative Clear under Track at B0405/B0506
and two omitted Track crossings: B0315 east and B0413 northwest into Jalo.
Jalo's palm-and-pool glyph is in B0513; its lettering in B0514 is not a second
location. A source-template variant at the unchanged 0.88 threshold recovers
its measured footprint center (3931.5,4211.5), independently of operational
water yield or capacity. The full-source candidate search finds only that one
additional match; this is not complete Oasis recall. Desert/plain boundaries
remain partial occurrences. Large Libyan Sand Desert letters are not ridges
or slopes despite their shared ink. The southern edge Tracks at B0111/B0114/B0117
and B0401's western continuation remain boundary findings, not invented links.

The next 165 cells B0601–B1033 add fifteen affirmative Clear occurrences and
the missing **B0707 east → B0708 Track** at Augila. The source fork lies northwest
of Augila's measured dot; neither it nor El Ergh's fork is snapped to a location
or given operational junction authority. Eight broad olive stippled band cells
(B0807, B0904, B0908, B1003, B1006–B1008, B1010) remain untyped: their shared ink
does not establish Mountain, Rough or homogeneous Clear. Enlarged controls
separate plain interiors from bands beyond neighboring grid edges, while retaining
boundary-footprint/direction uncertainty. The southeastern Track continuation
at B1033 leaves the retained inventory and remains a seam-owner finding.

The B1101–B1533 batch adds 46 Clear occurrences around Tracks/Wadi and a small
Rough lobe entering B1333 above its southwest grid boundary, not blanket Clear.
Two missing Tracks are restored: **B1233 northwest → B1333** and
**B1429 northeast → B1530**. B1507's previously corrected corner crossings are
retained, with affirmative Clear ground now recorded separately. Existing Salt
Marsh and Rough classifications remain unchanged; sixteen olive-banded cells
remain untyped. Wadi curves across upper B1529–B1533 do not trace exact grid edges:
their current side records are preserved with explicit association uncertainty.
Track/Wadi overlap does not imply a bridge or passage permission. Eastern Track
continuations from B1133/B1233 remain seam-owner findings. The prior B06–10
plain-ground source reference was widened to cover full edge-cell outlines;
its fifteen observations and per-cell source pins are unchanged.

The B1601–B2033 batch adds 93 affirmative Clear occurrences beneath Tracks,
area lettering and Wadi overprint. All 165 outlines were inspected individually,
with nineteen enlarged controls; no new settlement glyph or missing retained
Track crossing was found. Four existing Rough and 23 Salt Marsh occurrences
remain unchanged. HAMMIM AL WADI lettering is not a settlement. Wide Wadi
ribbons and olive bands retain unresolved edge associations and directional
interpretations; the isolated yellow slash at B1620 remains unidentified.
The northwest Track continuation from B1901 exits the retained B inventory
and is reported to the seam owner without inventing a zero-row neighbor.

The B2101–B2533 batch adds sixty affirmative Clear occurrences and restores
**B2217 northeast → B2318**, **B2229 northeast → B2330** and
**B2528 east → B2529** Tracks. Enlarged corner controls reject detours through
B2218, B2230 and B2628; existing Rough under B2318's Track remains unchanged.
Seven olive-band interiors remain untyped. Gravel circles straddling the western
boundaries of B2302, B2402 and B2503 also remain untyped pending glyph-footprint
interpretation rather than blanket Clear or Gravel. B2101's western Track
continuation remains a seam-owner finding. Existing Wadi side observations are
preserved with explicit association uncertainty, including outside/near-edge
ink at B2406 and B2511. No new settlement or facility glyph was observed.

The B2601–B3033 batch adds sixty affirmative Clear occurrences and four Tracks:
**B2604 northeast → B2705**, **B2718 northeast → B2818**, **B2806 east → B2807**
and **B2807 northwest → B2907**. Twelve-times nearest-neighbor controls distinguish
the B2718 direct crossing from the resolution-sensitive short B2807 wedge;
the latter rejects direct B2806 northeast → B2907. Original geometry is unchanged.
B2602/B2701 remain untyped where Gravel circles straddle boundaries. Existing
Giuf el Mater and Sauhu measured dots match source; Village/Bir category and
Sauhu label transcription remain uncertain. Three Rough, one Gravel and two
Salt Marsh occurrences stay unchanged. Wadi ribbons retain provisional side
associations, not exact boundary geometry. B2601/B3001 west and B3033 northeast
continuations are seam-owner findings, not fabricated cross-section links.
No new settlement or facility glyph was observed in these 165 cells.

The B3101–B3533 batch adds fifty-eight affirmative Clear occurrences and restores
**B3208 northeast → B3309** and **B3333 northwest → B3432** Tracks. Enlarged
controls reject detours through B3209 and B3433. Existing Maaten el-Grara and
Bir measured dots are retained with unresolved Village/Bir categories; Trigh el
Abd lettering is not a settlement. Three olive-band interiors B3102/B3201/B3501
remain untyped; nine Rough and five Salt Marsh occurrences remain unchanged.
Wadi associations stay provisional, including outside/near-edge ribbons.
Northwest/southwest Tracks in B3101/B3301/B3501, B3201 west and B3233 southeast
exit retained inventory and require seam-owner reconciliation. No zero-column
cells, B3134, cross-section links or operational permissions were invented.
No new settlement or facility glyph was observed in these 165 cells.

The B3601–B4033 batch adds forty-seven affirmative Clear occurrences and restores
**B3903 east → B3904** and **B4004 east → B4005** Tracks. Enlarged context rejects
a detour through B4003 and preserves the separate Msus southeast spur to B3905.
The Msus dot overlaps the horizontal route's start; the crossing remains
provisional, not an exact path or operational junction. Existing Msus and Ben
Gania measured dots and labels remain unchanged, with Village/Bir category
uncertainty. The diagonal branch northwest from Ben Gania through B3814, B3914
and B4013 repeatedly approaches printed vertices: 12× controls cannot consistently
resolve direct sides versus small neighboring wedges. Its presence is recorded,
but no new sides were forced in this batch. B4113 was context-only until the next batch.
B3701 olive interior and six Gravel-boundary cells remain untyped; seven existing
Clear cells retain explicit Gravel-footprint uncertainty. Seventeen Rough, eight
Salt Marsh and three Gravel occurrences stay unchanged. Wadi and olive bands
retain unresolved exact footprints and direction; no bridge or Up/Down inferred.
B3701/B3901 western Tracks and B3833 northeast leave the inventory and need seam
reconciliation. No new settlement or facility glyph was observed in these 165 cells.

The B4101–B4533 batch adds fifty affirmative Clear occurrences and eight Tracks:
**B4013 northwest → B4113**, **B4113 northeast → B4213**, **B4223 northwest → B4323**,
**B4223 east → B4224**, **B4426 northwest → B4526**, and the three eastern crossings
**B4429 → B4430 → B4431 → B4432**. Reconciliation of B4113's full outline with
B4013's northern 12× control resolves this endpoint left of the shared vertex,
not through B4114. B4013's southern endpoint and the remaining Ben Gania chain
remain unresolved. B4013's prior review is amended without extra coverage;
B3914's note clarifies that its uncertainty persists.
Existing Bir Tengeder and Rotunda Segnali measured points and labels stay unchanged,
with Village/Bir category and dot/route overlap uncertainty. Six Gravel-boundary
cells remain untyped and eight existing Clear cells retain footprint uncertainty.
Thirty Rough, twenty-eight Gravel and one Salt Marsh occurrence remain unchanged.
B4101 west, B4301 northwest, B4333 east and B4533 east are boundary findings for
the seam owner, not invented cross-section links. No new settlement or facility
glyph was observed in these 165 cells; watercourse effects remain unadjudicated.

The B4601–B5033 batch adds six affirmative Clear occurrences, partial Mountain
cores at **El Abiar B4901 and B5005** without replacing Rough, and eight Tracks:
**B4805 east, B4809 east, B4817 northwest, B4901 east, B4902 east, B4917 east,
B4921 east and B4921 northeast**. Paired 4× controls distinguish the B4817 branch
through B4917 from a detour through B4918, and Mechili's northeast branch into
B5021 from its east branch through B4922. One independent **B4901 east Railroad**
crossing is restored above the dashed Track. The existing **B4903 northwest
Railroad** may instead represent a transverse tie touching the shared vertex;
it is retained with explicit unresolved evidence, not silently removed.
Charuba, El Abiar, Mechili and Gazala keep their measured dots, labels and
Village/Bir uncertainty. No new settlement/facility glyph was observed in this
batch. Nine Gravel-boundary cells, broad-band B4833 and coastal B4933/B5032 stay
untyped. Existing 108 Gravel, 32 Rough and partial Salt Marsh B5033 occurrences
remain unchanged; six earlier Clear cells retain fringe uncertainty.
El Abiar's western rail/Track and northwest/southwest Track branches, B5001 west
Track and Gazala B4933 east Road remain boundary findings for the seam owner.
Gray section lines, broad Wadi and stippled side bands do not establish new
adjacency, permanent water, operational junctions or Up/Down effects.

At the B46–50 checkpoint, source-path regeneration associated the El Abiar
Railroad observation with an existing measured curve. All 158 curve geometries
and 38,042 raw samples were unchanged. Matched completed crossings totaled
555/588 (365/375 Road and 190/213 Railroad); 33 remained unmatched, including
the disputed B4903 northwest.
Tracks remain schematic; this does not resolve rail/Track visual overlap.

The B5101–B5531 batch reviews **159 retained cells**, not a rectangular 165-cell
range. Four affirmative Clear occurrences at **B5229, B5426, B5427 and B5527**
remain separate from Wadi, location and unfinished-Road overprints. Partial
Mountain cores at **B5301, B5315 and Chaulan B5518** do not replace existing Rough
or turn stippled bands into ground. Sixteen missing Track crossings and two
Railroad crossings restore the Barce approach and branches near Maraua/Martuba.
Paired controls place the B5316 route through B5216–B5217, not B5317; the B5515
Track joins the Road in B5614, not B5514. Tocra's Track crosses northeast into
B5601 independently of its east Road. At that checkpoint, B5601/B5614 and other
B56 crops were context-only and **not_reviewed**. The exact B5508 northeast/east-vertex route
toward El Garrib is still unresolved; no side is forced from endpoint connectivity.
The Track across B5523 east remains independent of the Martuba unfinished Road,
which is not promoted to a usable Road. Eight measured Village/Bir dots and all
prior classifications are unchanged. Gravel-boundary B5129/B5130/B5228, five
stippled-band interiors and mixed coastal cells retain unresolved ground/footprint
findings. Tocra B5501's western Road exits retained inventory and remains a
boundary finding for the seam owner. No precise centerline, operational junction,
port, Up/Down effect or section adjacency is inferred.

At the B51–55 checkpoint, Barce regeneration associated B5403 northeast with an existing rail
curve; B5202 northeast remains untraced and retains an amber incomplete-curve
marker rather than an invented chord. All 158 curve geometries and 38,042 raw
samples were unchanged. Matched completed crossings totaled 556/590 (365/375
Road and 191/215 Railroad), leaving 34 unmatched. Neither parallel Track overlap
nor missing source-curve coverage is resolved by this audit.

The final northern B batch reviews **152 retained B5601–B6123 cells**, completing
the section's 1,961 whole-cell source inspections, not its interpretation.
Six Track crossings restore the Tocra–Tolmeta coast and El Gubba's southbound
near-edge bend through **B5819 → B5719 → B5720**, not a direct shortcut.
Two continuous Road crossings restore **B5717 east and B5919 east** independently
of nearby Track and unfinished-Road capsules. Clear occurrences at B5627/B5629,
partial Rough at B5720/B5728 and a Mountain core at B5915 supplement existing
classes without replacing them. No Railroad is observed in this batch.

The omitted **Derna anchor at (4962.5, 253.5)** is separate from its existing
Village/Bir dot. A new source template at the unchanged 0.88 threshold also
recovers **Rosetta's anchor at (12978.5, 1565.5)**, independently spot-inspected in
E4120. Both retain unresolved operational port association; E4120 remains
`not_reviewed` for whole-cell coverage. Prior measured symbols are unchanged.
Coast slivers in B5923/B6020/B6021 visibly disagree with the raw detector's `land`
label; this is recorded without rewriting measurements or geometry. B5906/B6121
retain unresolved raw surface and explicit mixed-shore findings. The B5508/B5608
vertex, band polarity, coast ownership, construction completion and operational
effects remain unresolved. No seam adjacency or exact source path is inferred.

Integration regeneration adds one 71-sample Road curve east of El Gubba, matched
to B5919 east. All 158 previous curves, their samples and references are unchanged.
The display now has 159 measured curves and 38,113 raw samples, matching 557/592
completed crossings (366/377 Road and 191/215 Railroad). B5717 east remains among
the 35 unmatched crossings, with an incomplete-curve marker rather than a chord.
Track routes remain schematic and independent of construction.

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
