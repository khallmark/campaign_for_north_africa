# Source watercourse display audit

This is a display audit against the fingerprinted CNA map and TEC, not a
per-hex terrain survey or an operational river classification. It began on
`add8fb4a8f4aec0079f1eb5b916a6d58f35a852c` (the combined 7,076-cell baseline).
The private scan, SQLite database and comparison crops are not release assets.

## Rendering contract

`scripts/trace-map-water.py` extracts separate water and dry-wadi source ink.
`data/map/source-water.json` retains raw contours, simplified rings, holes,
source coordinates and provenance. `source_water.rs` renders those filled
outlines, not hex-center lines or constant-width strokes. Water stays blue;
dry wadis stay gray-green. The layer remains provisional display evidence.

Outer water/wadi vertices now have quadratic corner transitions. Each tangent point
is at most one board pixel from its source polygon vertex and no farther than
halfway along its edge. This bounds rounding to subpixel changes (at most
0.5 board pixels from the adjacent segments), with no spline overshoot.
The underlying polygon simplification tolerance remains 0.75 board pixels.
These are separate error budgets, not a claim of 0.5px accuracy to the scan.
Each component and hole stays a separate closed ring with even-odd filling.
Hole boundaries stay exact: rounding thousands of tiny source-print voids adds
rendering cost without recovering a bank and can alter the overprint evidence.
No path joins disconnected ink. Escarpment rings retain their existing straight
segments and ragged faces; they are owned by the escarpment workstream.

## Reviewed extent and findings

Automated extraction covers the complete **14,310 × 4,632** source raster.
Visual screening compared five complete vertical strips (x = 0–2862,
2862–5724, 5724–8586, 8586–11448, 11448–14310; all y) against extracted
water/wadi contours. These are raster review strips, not section boundaries.
Native-scale comparisons then examined the following regions; coordinates are
`[left, top, right, bottom]` in original board pixels.

| Region | Source crop | Finding |
| --- | --- | --- |
| Malta | [0, 0, 1500, 1000] | Sea and coastline present; no missing inland watercourse identified in the crop. Blue labels are not rivers. |
| Western A wadi | [300, 3700, 1200, 4250] | Narrow sinuous channel and widths already present. |
| B branching wadis | [3400, 3200, 4500, 3850] | Branches, enclosed land and termini match the source; do not extend them through mountain terrain. |
| C border wadi | [6850, 2800, 7450, 3400] | Blue barred overprints cause genuine ink interruptions. Those marks are not additional blue rivers. |
| D inland/Qara | [8900, 3750, 9400, 4300] | Short wadis terminate at the source scarp/terrain edge. The Qara point interrupts one channel; no connector inferred. |
| D coast/Matruh | [9300, 1650, 10300, 2200] | Coast silhouette retained; port/text interruptions remain. Inland road/rail marks are not missing watercourses. |
| Nile delta | [12800, 1500, 14200, 2300] | Distributary shapes and changing widths present. Roads, rail, labels and narrow grid remnants explain the local gaps. |
| Southern Nile | [13500, 3700, 14000, 4250] | Main curved outline present; printed “The Nile” lettering interrupts ink. Recover visible blue islands, not the obscured channel. |
| Sidi Brabri fragment | [2750, 1450, 2900, 1580] | Small wadi island beside lettering was discarded by the old area threshold. |
| B coastal crossing | [5320, 760, 5450, 880] | Small wadi island beside the road was discarded by the old area threshold. |

The full-map screening found no wholly omitted major watercourse network.
That does **not** certify all small features, all 7,076 cells, unseen ink under
overprints, or water represented with other inks. Full per-hex model/survey
review belongs to the separate source-audit workstream.

## Small fragments: accepted and excluded

The previous 150px² outer-contour threshold discarded real ink between
overprints. All **192 water** and **2 wadi** candidate components with contour
area 20–150px² were examined in source contact sheets. Many blue candidates
were sea-colored interiors of calendar/holding-box lettering, not geography.

Only new small components whose bounding-box midpoint falls within the existing
source `Malta` or `Map A`–`Map E` zone polygons are admitted. The polygons are read
from `public/data/map/sources.json` and fingerprinted in the output; they are not
edited or interpreted as playability or sea/land classifications.

- **44 water fragments accepted; 148 marginal water fragments excluded.**
- **2 dry-wadi fragments accepted; no wadi candidates excluded by the zone gate.**
- Accepted water fragments by source zone: Malta 1, A 5, B 9, C 6, D 3, E 20.
  The two accepted wadis are one each in A and B.
- Components below 20px² remain excluded; no absence inference follows.
- All **309 original outline objects** are unchanged, including all original
  water/wadi rings and the 17 baseline escarpments.
- This lane's result is **276 water + 62 wadi + 17 escarpment outlines**.
  The escarpment owner may independently add contours when integrating.

Accepted examples include the wadi fragments at `[2813,1522,5,9]` and
`[5376,815,12,9]` (`x,y,width,height`), Nile lettering at `[13790,3919,8,12]`,
and delta crossing ink at `[13576,2404,7,11]`. Excluded examples include calendar
ink at `[9375,200,8,20]` and holding-box letter interiors outside map zones.
Large preexisting sea/background contours are not newly clipped by this gate;
source label holes and broad background printing remain a known limitation.

## Regression and reproduction

```sh
uv run --with-requirements requirements-map.txt python scripts/test_source_water.py
uv run scripts/trace-map-water.py --write
sha256sum data/map/source-water.json > /tmp/source-water.sha
uv run scripts/trace-map-water.py --write
sha256sum -c /tmp/source-water.sha
npm run check
npx playwright test tests/browser/terrain-art.spec.ts tests/browser/map-viewport.spec.ts --workers=1
```

Synthetic tests check ink separation, grid repair versus pale ground, and
identical small colored fragments inside a map zone versus a marginal Game
Track zone. Browser tests use independent source-pixel transects in the Nile
and A/B/C/D wadis, including the restored fragments and neighboring overprint
gaps. The A4232 fragment assertion failed on the unchanged baseline (both
interior samples were absent). The same browser contract exercises pan, zoom,
Original terrain and Geography toggles without requiring private scan tiles.

For combined generation, merge the water/wadi scope gate with the escarpment
owner's extraction changes, then regenerate once. Keep this lane's 20px²
channel threshold independent of the scarp-specific threshold and simplification.
Do not resolve JSON conflicts by choosing one entire generated file.

## Candidate verification, 2026-11-03

This is a bounded candidate for integration, **not a fully validated release**.
The final outer-ring-only renderer was tested with this fixed served pair:

- `public/wasm/cna_bg.wasm` and `dist/wasm/cna_bg.wasm` SHA256:
  `d79621510df3042d2757dd6cd9a74e4c5c6d088dfe1f2674d4c793818679fcb5`.
- `data/map/source-water.json` SHA256:
  `701faf0a9cd152753d718c05ea2d77ea677be492dc52ed7e0713bcaada9376d6`.

`npm run check`, all three synthetic source-water tests and byte-identical
regeneration passed. The fresh serial terrain-art/map-viewport run finished
**55 passed, 1 failed**. The failure was the original wide-DPR2
`tiles follow drag, cursor zoom and a bounded visible ring` case, with its
unchanged screenshots, assertions and 30-second deadline. Three isolated
reruns against the same served pair also failed: 70 tiles were not marked ready
after Home; failures included the five-second readiness predicate and the
overall deadline. All four viewport/DPR water-shape cases passed.

Same-orb comparison restored the exact `add8fb4` water renderer and original
309-outline data, built it, and ran the same original case. That baseline also
failed twice at the 30-second deadline during wheel/state operations before
Home. This does not prove the candidate has no performance regression.

A diagnostic-only comparison omitted the two explicit high-resolution captures
from both variants, retaining the fixture, assertions and timeouts. Baseline
passed in 20.8 seconds; candidate passed in 19.6 seconds. At Home+0, both had
70 requested/responded/finished images, all complete with nonzero natural
width, but unset `data-ready`; by Home+1 second all 70 were ready, still true at
+4 seconds. This probe points to delayed ready-state updates rather than
missing image requests or decoding in that run; it does not establish the
cause of every original timeout. The original screenshots were restored and
no tile-loader behavior, assertions or timeouts were relaxed for delivery.

Final 2× screenshots inspected Nile and non-Nile B1515 watercourses with
Original terrain on and off. A real-pointer clip inspected pan, zoom and
Geography/Original terrain toggles; encoding required 1×/10fps after two 2×
recording attempts failed. Private scans were disabled. The source-shaped
curves and separate wadi color remain visible; source overprint gaps, schematic
route crossings, and existing artwork/model uncertainty remain visible too.
The integrating thread owns acceptance with the original full browser suite.
