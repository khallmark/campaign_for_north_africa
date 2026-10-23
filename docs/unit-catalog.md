# Campaign for North Africa source catalog

`public/data/unit-catalog.json` is a static, versioned reference catalog for the Rust/Wasm frontend. It is **not a playable campaign database or a certified inventory of unique historical units**. Every entry has `playable: false`. The prototype's invented HP, action points and supply balances have not been promoted to official rules.

## Coverage

| Content | Rows | Source: one-based physical PDF pages |
|---|---:|---|
| Organization-at-Arrival identity rows | 1,011 | Commonwealth 116–132; Italian 147–161; German 162–166 |
| National characteristics profiles | 124 | Commonwealth 133–134 (43); German 137 (31); Italian 169–170 (50) |
| Tank/gun equipment rows | 81 | 138–140 |
| Aircraft load/configuration rows | 102 | 112–113, 144–145 |
| Squadron ground support counters | 132 | 182–184 |
| Naval counters | 20 | 183–184; fleet schedule 142 |
| Additional Italian counter-only formation rows | 41 | 182 |
| Other named/lettered support counters | 11 | 183–184 |
| Role/counter-family entries | 52 | Rules 4.22–4.23, air/naval rules |
| **Total entries** | **1,450** | Profiles are separate, not included in this total |

All entries and profiles have a sprite reference. There are 24 shared role illustrations: four unchanged original assets and twenty newly generated assets. This does **not** mean 1,450 unique historical portraits.

Both supplied 192-page PDFs and `docs/rules.txt` were inspected. Git LFS initially exposed pointers instead of PDFs; fetching the objects resolved this. The scan is the numeric authority. OCR text helps locate material but is damaged in tables. Chart/counter rows were visually transcribed. `source_files` records SHA-256 checksums of the actual source files. Citations use **physical PDF pages**, not printed folios. The early schema sample's rule citations were corrected to physical pages 7 and 8.

Duplicate characteristics sheets on 167–168/171 and weapons sheets on 172–173 are not double-counted. Organization diagrams on 135–136/174–175 are not extra deployments. Generic convoys, motorization, aircraft mission markers, replacement pools and administrative markers are described in `counter_families`, not inflated into invented historical formations.

## What remains unknown

- Initial TOE, basic morale, weapon mix, arrival/withdrawal dates, map placement, readiness, pilots and supply inventories have **not** been transcribed. Their charts are often present; this is unfinished data work, not a claim that all those sources are missing.
- Sirte, Marmarica and Cirene appear on counter page 182 but their Organization-at-Arrival tables were not located in the supplied OA sequence. Their 41 rows have identities and illustrations but **no inferred profile or numeric characteristics**.
- Attached, replacement, alternative-organization and battlegroup rows can refer to the same historical unit. OA IDs identify source rows, not deduplicated counters. Some Italian battlegroups explicitly have no provided counters; German possible battlegroups outnumber the seven printed counters.
- The scan leaves Pz III E shifted cells, 7.62cm Pak/Marder III CPA, Cant Z506B range, Gladiator II fuel and German profile `v` close assault unresolved. Italian `pp` capacity is emplaced guns plus up to eight AA TOE, not one aggregate numeric maximum.
- Exact reconciliation of all 1,800 counter pieces and chart/counter spelling/identity aliases is not certified. A verified identity does not certify every deployment or establish a unique historical entity.
- Chart restrictions, refits, augmentations and scenario exceptions remain essential. Aircraft mission strings are transcriptions, not executable eligibility rules.

## Consumption contract

The JSON is read directly by Rust `serde_json`; there is no catalog logic crate or JavaScript runtime dependency. `docs/unit-catalog.schema.json` documents the v1 shape. Extra fields are allowed. Array IDs must be unique; references are foreign keys, not labels.

- `entries`: role families, OA source rows, counter formations, equipment, aircraft configurations, ground support, naval and other support units. `kind` is not the same as `unit_type`. Filter categories from actual data rather than assuming every entry is a formation.
- `profiles`: nationality-scoped printed ID codes such as `cw-h`, `ge-j`, `it-vv`. Includes `side` and an embedded sprite. A formation's `profile_id` points to **base characteristics**, not current scenario state. Do not automatically merge profile stats into a playable unit.
- `side`: `allied`, `axis`, or `unspecified`. Commonwealth includes its listed Allied contingents; US-labeled SGSUs are Allied.
- `verification`: transcription/identity confidence (`verified` or `partial` here). Numeric confidence is independent and lives on each stat.
- `stats`: map keyed by `stat_definitions.id`. Each value has `value`, `status`, `source_refs`, `note`, `raw`, and `conditional`.
- Only `status: "sourced"` contains numbers in v1. `unresolved`, `not_applicable`, and reserved `provisional` values are null. Zero is a genuine printed zero, not a substitute for a dash or missing cell. There are no invented provisional numbers in this release.
- `raw` retains the scalar notation, including parentheses and CPA markers where applicable. It is normalized transcription, not a facsimile of all printed footnotes. Show `conditional` and notes alongside the scalar. Halftrack armor and all-US tank-capacity exceptions are conditional even without parentheses.
- `breakdown_shift` is signed: positive = right, negative = left; `raw` displays `1R`, `1L`, etc. This is not a combat rating.
- Land `barrage` is not `naval_barrage`: naval pairs are actual barrage/AA points under 30.12. SGSU 20/25/30 numerals are CPA under 35.12, **not aircraft capacity**. Aircraft bomb points are explicitly **not tons**. Maximum TOE is **not HP**.

## Artwork and orientation

The image-generation tool (Painter) produced new transparent role atlases in the established miniature/isometric watercolor style, using the original sprite production approach. The first new atlas contained a 4×4 set and the second a 2×2 set. Alpha-connected components were assigned to the intended cell, with a two-pixel mask dilation preserving edge alpha, then trimmed and Lanczos-fit within 480×400. This avoided clipping neighboring figures where a simple grid cut failed. No white-background removal was used. Every image retains real RGBA alpha.

The four originals (`crusader`, `panzer`, `infantry`, `truck`) are byte-identical to the feature-branch baseline. Generated additions: Italian tank, self-propelled artillery, armored car, motorcycle, field artillery, anti-tank, heavy AA, machinegun, HQ, engineer, recovery vehicle, camel recon, fighter, bomber, warship, ground support, biplane, flying boat, coastal ship and desert raider. Shared proxies (e.g. bomber for transports, generic gun for artillery models) are deliberate and labeled illustrative. These are not scanned rulebook counters, exact historical equipment reconstructions, or a representation that the source rules are licensed for unrestricted reuse. No model seed was recorded.

`native_facing_degrees` describes the **existing single-view artwork**, clockwise from screen east, as an approximate artistic estimate: usually 150°, heavy AA 210°, flying boat 140°, desert raider 145°. Infantry, machinegun, HQ, engineer and ground support are marked non-directional. These are presentation values, not historical facts.

**Directional turnarounds:** `public/data/directional-sprites.json` separately describes all 24 shared roles, each with 18 generated upright front/side/rear views (432 cells). Paths are `/sprites/directional/{sprite_id}.png`; each RGBA atlas is 1536×768, arranged row-major in six columns and three rows of 256×256 cells. Frame 0 is nominally screen-east; subsequent frames advance 20° clockwise. Select/crop a frame; **never rotate the image or subtract the single-view baseline**. Keep the original upright sprite as the loading/error fallback. The original single-view files and catalog JSON remain unchanged.

The atlases are illustrative hand-drawn turnarounds, **not calibrated 3D rotations**. Nearby views can be nearly repeated, angular spacing is uneven, and equipment details can vary. HQ uses a single command officer instead of the static map-table group. Elevated AA barrels, engineer head turns and warship bow/stern outlines leave some bearings ambiguous; the manifest records those limitations. Do not infer simulation facing from the artwork.

Painter generated the alternate perspectives; incorrect quadrants were corrected through image edits or new small directional strips. Truck frames 5/6 and 11/12, Italian tank and self-propelled gun frames 13/14, and selected aircraft views were reordered. Bomber, biplane and flying-boat rear views required separately generated strips. Those strips were normalized to the original sheet's scale before packing. Processing only extracts, scales and positions genuine generated views: no mirrored substitutes or flat image rotations. Each silhouette is centered at x=128 with its bottom at y=240, an approximate ground-contact anchor rather than a rigged pivot. Generation source URLs and replacement-strip provenance are in the manifest.

## Validation

The schema can be checked with a standard JSON Schema 2020-12 validator, for example:

```sh
uvx check-jsonschema --schemafile docs/unit-catalog.schema.json public/data/unit-catalog.json
```

The delivery validation also checks unique IDs; all source, profile, type, stat and sprite references; source checksums; coverage counts; `playable: false`; and all 24 PNGs for dimensions, RGBA format and alpha extrema 0–255. The original four are compared byte-for-byte against the required baseline. Source spot checks cover genuine zero vs dash, asymmetric offense/defense values, left/right breakdown shifts, conditional armor, unresolved CPA, naval pairs and the three SGSU CPA values. Mutation controls reject changing unknown to zero, sourced to null, or a catalog row to playable.

Light/dark-background proofs of all 20 new sprites were visually inspected for clipping, neighboring sprite fragments, rectangular residue and edge halos. These checks validate assets/data, **not game behavior**. Rust parser/build/browser integration belongs to the parent integration thread; no frontend, simulation or build files are changed by this catalog delivery.

All 24 directional atlases were checked for fixed dimensions, RGBA alpha extrema 0–255, 18 nonempty distinct cells, transparent cell margins, and matching manifest IDs/paths. Light/dark composites were inspected for upright views, gross heading errors, clipping and halos. These checks do not establish exact angular spacing or historical accuracy. The rotation owner separately exercised the original four atlases through all seven active Rust game units and inspected recordings for upright artwork, horizontal counters and cell bleed; the additional twenty catalog-only roles have not been exercised in gameplay.
