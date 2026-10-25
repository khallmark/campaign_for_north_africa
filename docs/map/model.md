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
package is required for this importer.

```sh
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
into the existing `MapRecord` vocabulary. The importer verifies the original
map, TEC and module XML SHA256 values, and stores fingerprints of the census,
calibration, schema, exporter, vocabulary, surveys and importer. A changed source
requires review, not a blind hash update. Correct geography in surveys, retaining
source crop references, review status, reviewer, date and explanation, then
regenerate. Keep conflicting interpretations unresolved until adjudicated.

The browser uses the same Rust geometry and survey inputs as SQLite; it does not
download SQLite. Search accepts only inventory members. Inspect mode is separate
from counter placement; click inspects the pointer cell and Enter inspects the
viewport center. Dragging still pans. Grid outlines appear at zoom ≥0.22, labels
at ≥0.65. Source scan can be hidden to inspect the independent vector model.
Overlapping section identities are separately searchable; pointer hit testing
retains the existing section priority and does not resolve seam identity.

## Coverage as of the first transcription batch

There are **19 surveys / 102 observations**: 70 terrain, 23 hexside and 9 point
observations. **96 are provisional, 6 unresolved, none human-reviewed.** Terrain
has observations in 70 cells (B 35, C 9, D 10, E 16); 7,110 cells have none. Even
observed cells remain partially transcribed; absent features are unknown, not
Clear or evidence of an absent road/river.

- Matruh: settlement dots, distinct connected coastal road/rail samples,
  Charing Cross unfinished railroad evidence, Gerawla track, rough and salt-marsh samples.
- Giarabub: village, two oasis symbols and salt-marsh sample. The oasis label's
  extent is not interpreted as a region filled with oases.
- Libyan Sand Desert, Jebel Akhdar and Nile Delta: interior samples, using the
  existing Desert, Mountain and Delta vocabulary after comparison with TEC.
- Five Nile minor-river side observations; no wadi/slope/escarpment transcription yet.
- Unresolved: Matruh's offshore port icon versus its operational settlement
  hex; Tanta's city category; four Charing Cross unfinished railroad sides.
  The disconnected cross marks match TEC's **Unfinished Railroad**, not its
  continuous Railroad symbol. These sides are excluded from typed network links;
  construction state and operational interpretation need a separate contract.
  No capacity or city level is assigned.

These are **samples**, not completed regions. Network ends are transcription
limits, not real termini. The unfinished road southwest of Charing Cross is
excluded rather than misclassified as a track. No movement costs, water amounts,
bridge rules, network capacities or cross-section routes are inferred.

## SQLite contracts for later simulation research

`hexes` contains section identity, center, JSON outline and terrain completeness.
`hexsides` contains canonical endpoints and JSON edge geometry. `observations`
joins surveys and provenance, with normalized `observation_sources` pointing to
hash-pinned `sources`. `coverage` reports per-section geometry/terrain counts.

`network_links` exposes both directions of each road/railroad/track/pipeline
crossing, including its observation ID, survey and review status. It supports
connectivity research, **not legal movement**. It is not a geometric centerline
or a claim that every pair of routes in a hex forms an operational junction.
Consumers must retain type, evidence status and incomplete endpoints, and need
separate reviewed movement/transfer rules. The overlay draws these center-to-center
links schematically; other hexside features follow the actual shared edge.

```sql
SELECT * FROM coverage;
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
