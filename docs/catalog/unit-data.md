# Supplemental unit data — first VASSAL import batch

Baseline: published [aa485cc](https://github.com/khallmark/campaign_for_north_africa/commit/aa485ccca7ebc76b4b56cdf02d88ebded552b64f), not the older main branch. This batch reuses the existing census and identity contracts. It changes neither shared Rust contracts, scenario state, application code, source locks nor existing catalog/identity JSON.

`public/data/unit-reference.json` is an independently versioned **reference sidecar**, schema version 1, described by `docs/unit-reference.schema.json`. It is not an executable scenario, a second census or a claim of complete historical verification. `data/sources/identity-matrix.json` is the additive CNA-138 reconciliation artifact; licensing is tracked under CNA-136 in [licensing.md](licensing.md).

## Loaded versus verified

| Content | Coverage and limits |
|---|---|
| Existing source catalog | All 1,450 records, 124 characteristic profiles, 53 unit types and 21 stat definitions retained; not newly verified in their entirety |
| Existing canonical candidates | All 1,356 IDs, source-row mappings and 18 redirects preserved; these remain provisional candidates |
| VASSAL census | All 2,468 GPIDs retained, including markers, OC slots, palette copies and layered state artwork; 1,307 distinct names are not 1,307 unique units |
| Token assets | 1,962 distinct referenced filenames; 1,948 present with source hashes/private-review URLs, 14 genuinely missing from the supplied module |
| Reviewed identity associations | 63 identities linked to 118 GPIDs; all 61 identities with ready generated art have token links |
| Unresolved associations | 1,293 canonical candidates and 2,350 source slots explicitly unresolved; the latter includes non-unit markers and duplicate slots, not just missing historical units |
| CPA comparisons | All 124 national-profile scalar values agree between the VASSAL charts and original scan pages 133–134, 137, 169–170; footnote glyphs and other columns are not thereby certified |
| Organization at arrival | 12 1st Armored Division rows: raw equipment/TOE notation, arrival and basic morale compared with original physical page 116 |
| Descriptions | One formation-level historical paragraph transcribed from VASSAL, associated with those 12 rows; no invented individual-unit biographies |
| Errata | Five cited corrections applied once in the sidecar, preserving original observations: Pz III E row, Pak CPA, Marder CPA, Gladiator fuel and Z506 range |
| Searchable source text | 255 chart/OC images OCRed, 3 PDFs text-extracted, 4 text documents imported; all 262 indexed with source hashes and tool versions |

OCR and PDF text are **unverified transcription aids**, kept in `data/sources/vassal-2.1-document-text.json`, never automatically promoted to structured numeric values. Extracting a PDF text layer does not read its image-only pages. The VASSAL scenario PDF's text layer is especially sparse. Saved-game files are never decoded.

## Consumer joins

1. Find `units[]` by existing canonical `id`, resolving `identity_redirects` first if necessary.
2. Follow `counter_gpids[]` into `counters[]`; select `basic_image_id` or inspect the preserved `layers`/`image_ids`. These are source representations, not an inferred active scenario state. Keep separate slots rather than deduplicating on labels.
3. Resolve the filename in `token_assets[].id`. `public_path` is always null. Present assets have an encoded `private_review_path`, `/__source-tokens/<basename>`, and original `source_path`, SHA-256 and size. Missing assets have null hashes/paths and `exists:false`.
4. `generated_art` is the unchanged inventory sprite object. Its ready portrait/atlas paths are illustrations, never original tokens. Missing artwork remains missing, not a token or shared proxy silently relabeled as unique art.
5. Follow `source_record_ids` to `records[]` and `characteristic_profile_ids` to `profiles[]`. Show `raw`, `conditional`, `note`, source citations and cross-check status alongside values. Do not flatten multiple profile assignments into one current unit rating.

`unit_type_ids` are source-catalog types, not sandbox capabilities. `organization_at_arrival` and `description` are nullable. Equipment quantities remain raw **TOE points, not vehicle counts**. `N`, parenthesized morale and arrival notation such as `3/58` remain source notation; no calendar conversion or scenario initialization is invented. Description `scope` explicitly says `formation_history_not_individual_unit`.

CPA is an allowance, not the prototype's remaining action points. Rules 6.11–6.17 distinguish a genuine zero, combat-only use, motorization and per-stage expenditure. A profile's `10+` retains base CPA 10 and its motorization note; it is not an automatically increased allowance. Unknown and inapplicable values stay null. Positive/negative `breakdown_shift` means right/left shifts, not combat strength. Pz III E's corrected zero BAR remains conditional because the earlier German-tank date rule still applies.

Each `cpa_checks[]` item stores both VASSAL raw/scalar and original-catalog raw/scalar observations. Disagreement produces `result:conflict` without overwriting the original stat. These scalar comparisons do not reconcile degraded dagger/star glyphs. Erratum-corrected cells include `original_observation`, exact quote/locator, source hash and `verification:explicit_vassal_erratum`, rather than claiming that the original printed row already contained the correction.

## Reproduction and validation

Python 3.9+ is required. Source files must be hydrated, not Git LFS pointers.

```sh
python scripts/import-unit-data.py
python scripts/import-unit-data.py --check
PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -s scripts -p test_unit_data.py
uvx check-jsonschema --schemafile docs/unit-reference.schema.json public/data/unit-reference.json
bash scripts/check-sources.sh
bash scripts/check-no-credentials.sh
```

Optional text refresh, requiring Tesseract and Poppler:

```sh
OMP_THREAD_LIMIT=1 python scripts/import-unit-data.py --ocr --workers 4
```

OCR cache reuse requires the same source hash and tool version. Structured import is deterministic and `--check` is read-only. Original manual file hashes, module XML, token files, chart files, text sources and errata quotes are checked; a source change requires review, not silently updating source locks. The original catalog, census and identity inventory are inputs and remain byte-identical.

Nine tests cover retained identities/redirects/artwork, every source slot, foreign keys, adversarial equal-name and wrong-board matches, palette image mismatch, one-time errata, zero/null/conditional distinctions, source-image gates, OA scope and deterministic CLI output. No new Rust or browser behavior is claimed by these tests. Cargo is unavailable in this data orb, so the application build/tests have not been rerun for this batch.

Parent-reported baseline evidence, not fresh data-side testing: batches18–19 passed six searched deployments and token-independent generated-art path/movement checks; 27–28 sampled travel positions, 16–18 stationary turning samples, one opaque heading during travel, retained nodes and no rotations. Six screenshots were inspected; only 1/7 Queens had inspected 15fps footage. Earlier parent tests/build/schema checks passed on that baseline. Timing comes from DOM samples, not the footage. Source-art angle/detail jumps and the Frontier depot label overlap remain. The parent now uses nearest opaque views at 2ms/degree, minimum80ms, turning in place before travel; this importer does not alter or validate that animation.

## Remaining work

The sources are substantially richer than the structured verified output. Most OA rows still need equipment, TOE, morale, arrival/withdrawal, notes and identity reconciliation. Broader weapon/aircraft profile comparison, time-dependent refits, attachments, battlegroups, aliases and missing images remain open. Original scans are degraded in places and sometimes disagree with VASSAL layout; preserve both observations. There are still 1,295 candidates without unique generated art. Do not label all slots, all source stats, canonical identities or artwork complete.
