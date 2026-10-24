# Batch18/19 reconciliation — CNA-139

Canonical leaf: `cna-catalog:E1.I1.S1`. Investigation date: 2026-10-24.

**Later private integration checkpoint:** Kevin subsequently requested “Push EVERYTHING” to the now-private repository. The integration branch now includes all twelve batch18/19 PNGs and the 61-set inventory, preserving the newer implementation and source files. This supersedes the pending integration decision below, not its historical-fidelity or gameplay-review limitations. Main is unchanged; the following investigation records the earlier baseline.

## Decision requested

Kevin: approve or decline merging **batch19 only**, which already contains batch18, into the integration baseline. This note prepares that decision; it does not authorize or perform the merge. Do not merge batch18 separately, replace the checkout with the old branch tree, or regenerate these six sets.

Current merged coverage remains **55 ready sets / 1,301 missing** among 1,356 provisional candidates. The proposed merge would yield **61 ready sets / 1,295 missing**, not 61 newly certified historical units. Distinct units require distinct sprites; versions of a unit use counters/symbols.

## Exact source revisions

| Source | Revision | Ready sets |
|---|---|---:|
| Checked-out baseline and fetched `origin/main` | [5b8f992](https://github.com/khallmark/campaign_for_north_africa/commit/5b8f99288c2f1c6152397b366462373a58050b3f) | 55 |
| `origin/feat/identity-art-batch-18` | [c9aceb1](https://github.com/khallmark/campaign_for_north_africa/commit/c9aceb1367e13ca6ca40b4d196de6a409c821fd7) | 58 |
| `origin/feat/identity-art-batch-19` | [bb70692](https://github.com/khallmark/campaign_for_north_africa/commit/bb70692374eacbc16cf80560f7d13f548ed43533) | 61 |

The common ancestor of current main and batch19 is [41adbb6](https://github.com/khallmark/campaign_for_north_africa/commit/41adbb690bc4b395b53727cc77b91a89c87af5e1). The two art commits form a stack on that older baseline. A two-dot tree comparison includes unrelated later main additions; the three-dot branch diff isolates the actual art changes: two metadata/doc files and twelve new PNG files. Preserve newer main files and any unpublished parent work during an eventual approved integration.

The [original artwork thread](https://ampcode.com/threads/T-01a1637c-7f4c-75fc-b803-9c26efd62b46) records generation, corrective strips and bounded parent-reported gameplay checks. The remote branches, not older unpushed archive descriptions, are the inspected asset sources.

## Six existing identities gain artwork

All six IDs already exist on main with `sprite.status = missing`. Batch19 changes them to `ready`; it adds no identities and collides with none of the 55 already-ready IDs.

| Batch | Exact identity ID | Files under `public/sprites/units/<id>/` |
|---|---|---|
| 18 | `unit-commonwealth-46th-royal-tank-regt` | `atlas.png`, `portrait.png` |
| 18 | `unit-commonwealth-50th-royal-tank-regt` | `atlas.png`, `portrait.png` |
| 18 | `unit-commonwealth-2nd-derbyshire-yeomanry` | `atlas.png`, `portrait.png` |
| 19 | `unit-commonwealth-1-5-queens` | `atlas.png`, `portrait.png` |
| 19 | `unit-commonwealth-1-6-queens` | `atlas.png`, `portrait.png` |
| 19 | `unit-commonwealth-1-7-queens` | `atlas.png`, `portrait.png` |

The inventory additionally updates gameplay-review metadata for three existing batch17 sprites: `unit-commonwealth-40th-royal-tank-regt`, `unit-commonwealth-7th-rifle-brigade`, and `unit-commonwealth-11th-krrc`. These are evidence updates, not extra artwork sets. No non-sprite identity fields, roles, source-row maps, redirects, catalog hash or source catalog data change.

The branch documentation adds batch17 verification, batch18/19 provenance and historical delivery notes. Some older paragraphs still say unpushed or awaiting gameplay checks; later paragraphs supersede those historical states. The present publication state is six published but unmerged sets. This leaf corrects the baseline's stale introductory 49/1,307 count to 55/1,301, without importing the branch inventory. An eventual merge may conflict at that same sentence: resolve it to 61/1,295 only once the inventory and all twelve PNGs are actually integrated, retaining the publication and review caveats.

## Verification and limits

Executed against the fetched revisions, without checking out or merging either art branch:

- `git merge-base --is-ancestor origin/feat/identity-art-batch-18 origin/feat/identity-art-batch-19`: exit 0.
- `git diff --name-status origin/main...origin/feat/identity-art-batch-19`: only the two docs/data files and twelve PNG additions listed above.
- `git diff --exit-code origin/main origin/feat/identity-art-batch-19 -- public/data/unit-catalog.json`: exit 0.
- Python comparison of both inventories: 1,356 unique IDs in each, identical ID sets and non-sprite entry data; six missing-to-ready transitions and three ready-to-ready review updates.
- Pillow inspection of Git blobs for all 61 branch-ready sets: 122 distinct canonical-path files exist; every atlas is 1536×768 RGBA and every portrait 256×256 RGBA, with transparent and opaque alpha values. All 1,098 cells are nonempty, have transparent margins, and are pixel-distinct within their atlas; each portrait equals zero-based atlas frame7.
- `uvx check-jsonschema --schemafile docs/unit-identities.schema.json public/data/unit-identities.json`: passed on the 55-set working baseline. The same schema also passes against the batch19 inventory exported to a temporary file, without modifying the working inventory.
- `npm test`: 25 tests passed (11 frontend and 14 simulation behavior), zero failures or ignored tests, on the unchanged 55-set application baseline. This is not an application test of a merged batch19 build.
- `git diff --check`: passed for this leaf's documentation change.

These checks establish file structure and data consistency, not calibrated headings, historical fidelity, walking animation, rights clearance or a fresh gameplay review of the six assets. No sprites were generated, changed or imported. Source PDF files in this orb are currently Git LFS pointers rather than hydrated scans; historical-source review must obtain the real files before using them as evidence. This does not affect the Git-blob PNG checks above.

## Delivery boundary

Only this note and the stale introductory count in `docs/unit-identities.md` belong to CNA-139. The refreshed planning snapshot and mappings are transferred parent work, kept separate. No pushes, PRs, merges or deployments are authorized by this leaf. Catalog census CNA-137 and contract CNA-141 require simulation scaffold CNA-346 to be complete in Linear **and** its exact verified code to be present locally before they start.
