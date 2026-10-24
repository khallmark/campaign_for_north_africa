# CNA planning: Claude pass and Codex reconciliation

Canonical files: `cna-map.plan.json`, `cna-unit-movement.plan.json`, `cna-catalog.plan.json`, `cna-simulation.plan.json`, `cna-dag.manifest.json` (cross-project edges, stages, ownership, tool prerequisites, decisions) and `cna-dag-check.py` (global validator).

| Project | Epics | Issues | Leaves (S/M) |
|---|---:|---:|---:|
| cna-map | 6 | 11 | 39 (13/26) |
| cna-unit-movement | 5 | 8 | 16 (8/8) |
| cna-catalog | 7 | 16 | 49 (19/30) |
| cna-simulation | 9 | 83 | 272 (134/138) |
| **Total** | 27 | 118 | 376, 80 cross-project edges |

`devkit pipeline validate --stage full` passes for all four plans; `python3 docs/planning/cna-dag-check.py` passes. Mutation runs confirmed it rejects a local cycle, a leaf blocked by its own parent, a backward cross-project stage edge, a file edited by a non-owner, a missing harness prerequisite, a missing bin and an L-sized leaf.

## Global order

Stage 0: simulation E1 scaffold (workspace members, `npm run test:browser`, owner file split, `scripts/check-sources.sh` and `scripts/check-no-credentials.sh`); map E1 and catalog E1 contracts (they need only the scaffold). Stage 1: simulation E2 contracts, ledger, adjudications and per-section comparisons; map tiles/calibration; catalog adjudication; movement fixtures. Then foundation (sim E3), detailed logistics with maneuver (sim E4), land combat (sim E5), Graziani acceptance (sim E6), air and abstract modes (E7), remaining scenarios (E8) and simulation-owned integration (E9, stage 8). No producer depends on a consumer; corrections are new forward leaves.

## Codex findings adopted

- Maps A–E plus Malta as a separate sixth grid; seam cells are one canonical HexId with printed aliases so seams never double adjacency or cost; D2228/2328 escarpment is one final hexside record and is not added again to the already-corrected raster (map E3.I1.S2, E4.I2.S4, E5.I1.S9).
- Viewport budget targets (≤128 MiB decoded cache, ≤100 tile nodes, p95 pan ≤33 ms, DPR 1/2) recorded as targets to measure, not claims (map E2.I1.S2).
- §11.32: adjudication record with the 108→11 example, summation before rounding, raw 4/5 and .4/.5 boundaries and the 11.37 AA exception as fixtures (sim E2.I3.S1, E5.I1).
- Shipping cadence as a state-machine decision before cadence code (ADJ-SHIP, sim E2.I3.S2; E4.I17.S2 integration).
- §60.45 fleet inventory/release, Cairo vs Cairo/Alexandria supply, §63.2 duration, errata idempotence, mode matrix and VASSAL editorial OOB choices as eight adjudication records (sim E2.I3).
- Unknown reference data stays unresolved (typed `UnresolvedReference`), never a sandbox default; Sirte/Cirene/Marmarica quarantine certification.
- Historical replay privacy (sim E3.I3.S3), hidden-state equality testing across previews/errors/logs (E3.I3.S1), and one fixture per L21 3.62 item (E3.I3.S2).
- Distinct abstraction modes with one accounting kernel per resource (sim E3.I6.S1, E7.I16.S2).
- Allowlisted in-memory save import that drops PLAYER/password records first (sim E3.I5).
- `Inventory::find` ignores `identity_redirects`: fixed in catalog E1.I2.S1. Stale "49 ready" text corrected in E1.I1.S1.
- Movement event cursor/revision handling, no unaccepted suffix playback, and seam-edge perpendicularity (movement E2.I1.S1, E4.I1.S1).
- Global stage and parent/child cycle validation, plus prerequisite harness leaves for every verification command (validator `tool_prerequisites`, generic bin/test/schema rules).

## Codex proposals not adopted (deliberately)

- 280 fixed raster rectangles for the map: replaced by per-section terrain and hexside leaves (Graziani corridor C east/D/E first, then C west, B halves, A, Malta) with integrity tests and 10% independent spot checks. Rectangle batching can be added as append-only children if review shows sections are too large.
- 62 XML-slot batches and 49 reference-page batches for the catalog: replaced by the census (all 2468 slots), a cross-source matrix and four adjudication waves by nationality/kind. Data transcription is bounded by scenario need (Graziani first) and by side.
- At-most-two-identity art leaves: Wave A is 13 fixed leaves pinned to SC21 60.31/60.41 entries; waves B–F get explicit materialisation leaves that append ≤8-identity leaves after the freeze, and the coverage gate blocks until they exist.

## Material decisions in this pass

- Every rules section 1–65 has a compare leaf (E2.I4–E2.I8) plus, when normative, its own issue with a source-derived fixtures leaf (tests written first, ignored until implemented) and implementation chunk leaves. Sections 1, 2 and 65 are explicitly not_applicable. 3.6 is implemented in the projection engine.
- Seven image-only page groups get transcription leaves with an independent second pass before any setup uses them.
- Graziani's Offensive (60.22, 18 OpStages) is certified only in DetailedLogisticsAbstractAir58 mode, on map-v1.1 geography and non-fixture catalog data; status is "provisional" while any used adjudication is pending.
- The sandbox is moved, not modified, into `cna_sim::sandbox`; official rules never import it.
- Simulation owns all shared entry points; per-owner modules and CSS files are created once by sim E1.I1.S3 (recorded scaffold exception).

## Remaining blockers and decisions for Kevin

Listed in `cna-dag.manifest.json` → `decisions_for_kevin`: the eight adjudications, map-art redistribution, counter/generated-asset distribution, and approval to merge `origin/feat/identity-art-batch-19` (which contains batch-18; 55 + 6 = 61 sets). Cross-project blockers must be created as Linear relations from `cross_project_edges` in a second pass and re-read for equality; the devkit schema cannot hold them.
