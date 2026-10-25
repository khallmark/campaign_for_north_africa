# Committed SQLite catalog

`data/catalog.sqlite` is the **committed private reference database**, not a build
cache. VASSAL is the import source; numbered SQL files record reviewed corrections
and small curated changes. Commit the importer/SQL changes and resulting database
together. The existing JSON files remain compatibility inputs for the current
frontend; they are **not read by ordinary database imports**. The bounded
projections below update classifications and reviewed profile links from this database.

## Import and correction workflow

Python 3.11+ and SQLite 3.37+ are required. Source import also needs Cargo and the
hydrated, integrity-locked source checkout. No Python packages are required.

```sh
python3 scripts/import-catalog-db.py              # VASSAL first, then pending SQL
python3 scripts/import-catalog-db.py --check      # same import, detect drift, never replace
python3 scripts/import-catalog-db.py --patch-only # SQL only; no sources/Cargo needed
```

### Classification compatibility projection

After applying a classification patch, run:

```sh
python3 scripts/import-catalog-db.py --project-types
python3 scripts/import-unit-data.py
python3 scripts/import-catalog-db.py --project-types --check
```

The first command reads the committed DB read-only and replaces only `unit_type`
in the existing catalog's `entries` and `profiles`. The existing JSON is a template
for every other field, not the authority for classifications. Schema version 1,
exact record IDs/order and the existing type vocabulary must match before any
write. No-op runs preserve bytes; drift checks never write; failed validation
leaves the output intact. This is not a full exporter or permission to add private
fields. It cannot recreate a missing catalog: restore the compatibility template
from Git. Profile assignments, stats (including unresolved/conditional observations),
affiliations, citations, sprites and IDs are not exported or changed by this mode.

The second command is the existing hydrated-source sidecar generator. It updates
record types, each canonical unit's sorted `unit_type_ids`, and the catalog input
fingerprint; it does not change token links or generated art. Commit DB, SQL,
catalog and sidecar together. Fast tests reject a stale projection or sidecar;
the existing source suite still verifies full sidecar regeneration.

Patch `002-reviewed-unit-classifications.sql` corrects twelve individual records
covering ten formerly empty categories. Twelve evidence rows retain the previous
classification, rule anchors and visually inspected VASSAL image/GPID locators.
It fails if any expected original record/type is missing or changed. Shared
profiles are deliberately unchanged (for example `cw-uu` covers both road and
rail construction). Historical motorization is not current truck availability.
Motorization points, replacement points and generic truck convoys remain without
a named historical-unit roster. Coverage is still partial; this is not a full
classification audit. The French Motor Marine company's token link stays unresolved.

### Reviewed characteristic-profile links

Patch `003-reviewed-italian-profile-links.sql` restores 41 explicit ID-code links
from the VASSAL organization charts for Sirte (14), Marmarica (13) and Cirene (14).
Each row was visually checked; these are not assignments inferred from unit names.
The three chart citations and per-record evidence retain the printed row/code.
Rule 3.5 is a textual characteristics-definition anchor, not evidence of the
individual mappings. The image paths refer to integrity-locked VASSAL sources.

```sh
python3 scripts/import-catalog-db.py --patch-only
python3 scripts/import-catalog-db.py --project-profiles
python3 scripts/import-unit-data.py
python3 scripts/import-catalog-db.py --project-profiles --check
```

This separate projection handles only records with `profile-link:<record-id>`
evidence. It copies their profile IDs, curated record notes, ordered citation IDs,
and the reviewed explanation of unresolved CPA. It appends missing citations
using an explicit public-field allowlist and retires the obsolete no-profile
coverage warning. It preserves ordering, identities, affiliations, artwork, all
numeric values, raw/conditional/status fields, and unknown-versus-zero distinctions.
Original observations remain unchanged in SQLite, including their historical
no-profile note; the projection replaces that obsolete explanation with the
reviewed evidence note. No private document text or arbitrary JSON details leave
the database. Validation precedes atomic replacement; `--check` never writes.

After this correction, 315 of 1,356 canonical candidates have direct numeric
observations, 1,015 have numbers only through linked profiles, and 26 have neither.
The latter include 19 Italian formation candidates and seven special/support
identities. Blank ID-code cells on the unassigned Italian tank chart are not
guessed. **Profile values are base characteristics, not current scenario values.**
The frontend must show them separately instead of silently merging them into a
formation's direct observations. The data correction alone does not change UI
rendering; consumers still require a Wasm rebuild.

The source command verifies `docs/sources.lock.json`, runs the existing Rust
`vassal-census` against `docs/CNAv2.1.0/buildFile.xml`, and loads its temporary
output. It does not reparse VASSAL independently or use the committed JSON census
as the import source. Images remain files: SQLite stores paths, hashes, existence,
layer labels and source locators, not image bytes or decoded save state.

Add `data/sql/patches/004-<description>.sql` (then 005, etc.). Filenames sort
lexically, so keep the padded numbering. Use `counter_correction` for corrected
counter labels, `stat_correction` for reviewed numeric/not-applicable overrides,
and `correction_evidence` for exact source path, locator, quotation and rationale.
Update curated unit records, descriptions and links directly when appropriate.
Do not merge identities just because names match. Preserve retired-ID redirects.

Source tables are replaced on re-import; **never fix them directly**. Patch DML
against raw VASSAL tables, original stat observations, source files/text and import
metadata/ledger is rejected. This is an accidental-edit guard, not a sandbox for
untrusted SQL: patches are trusted, reviewed repository code. Schema migrations
are not implemented by this slice; changing `schema.sql` after initial import
fails its digest check. Do not bypass it with a correction patch.

Applied filenames and SHA256 values are stored in `sql_patch`. Previously applied
files must remain an unchanged prefix of the patch list. Editing, deleting or
inserting an earlier patch fails. Correct an applied change with a **new** patch.
Patch-only mode can work offline; only a full source run checks quotations against
the source files. Its narrower validation is not a manual-verification claim.

Each run copies the committed DB into a disposable sibling directory, imports,
applies patches, checks integrity/FKs/IDs and source evidence, then atomically
replaces the destination only if logical content changed. SQL errors, missing
linked GPIDs, invalid redirects or evidence mismatches leave the original intact.
Fix a failed **unapplied** patch and rerun. An unchanged rerun leaves even the
database bytes untouched; `--check` fails on any pending logical change.
Comparison is schema/row based, not SQLite-version-dependent page layout.

Close other writers/checkpoint WAL before import. The importer rejects journal
sidecars and detects changes to the destination during its run. It is a
single-maintainer workflow, not a concurrent database server or locking protocol.

### One-time migration already performed

`--bootstrap` refuses to overwrite an existing database. It imports VASSAL first,
then preserves earlier reviewed work from `unit-catalog.json`,
`unit-identities.json`, `unit-reference.json`, `identity-matrix.json` and extracted
document text, then applies SQL. Their hashes are recorded under
`metadata.bootstrap_sha256:*`. This explains the initial manual transcriptions,
canonical identities and generated-art associations; it is not the normal source
of future edits. Restore the committed database from Git rather than bootstrapping
over curated work. Existing unresolved-matrix diagnostics remain in their JSON
artifact; absence from `unit_counter` is an unresolved link, not a rejected unit.

## Read contract (database version 1)

Open read-only with SQLite URI `mode=ro`; check `PRAGMA user_version = 1` and
`application_id = 1129202001`. Explicit `position` columns preserve original array
order; SQL row order without `ORDER BY` is not a contract.

| Read surface | Meaning |
| --- | --- |
| `canonical_unit`, `identity_redirect` | Stable candidate IDs and direct retired-to-live redirects; identity status remains provisional where unresolved |
| `unit_record`, `catalog_record`, `unit_type`, `stat_definition` | Ordered associations, manual records/profiles, role descriptions and stat definitions |
| `effective_stat` | Corrected value/status/raw/conditional/note, plus original value/status/raw and correction evidence ID; NULL is not zero |
| `stat_observation`, `record_source`, `source_citation` | Preserved manual observations and citations; no automatic promotion of OCR to verified data |
| `effective_counter` | Raw VASSAL fields and separate `effective_label`; candidate duplicate slots do not establish identity |
| `unit_counter`, `unit_token` | Reviewed identity links and original basic token metadata; `unit_token.label` is the raw source label |
| `counter_image`, `counter_layer`, `counter_level` | Additional/state images and ordered raw layer values; not an interpreted gameplay state machine |
| `generated_art` | Separate generated portrait/atlas/status; ready requires both paths |
| `unit_context`, `cpa_check` | Existing reviewed descriptions/organization and bounded manual comparisons, with evidence details |
| `source_document`, `source_file`, `correction_evidence` | Private extracted text, provenance fingerprints and correction quotations |
| `coverage` | Counts of linked/unresolved data and missing images; `complete = 0` deliberately remains explicit |

For example, query all linked record stats without assuming one counter per unit:

```sql
SELECT u.id, u.name, r.record_id, s.stat_id, s.value, s.status, s.conditional,
       s.original_value, s.evidence_id
FROM canonical_unit u
JOIN unit_record r ON r.unit_id = u.id
JOIN effective_stat s ON s.record_id = r.record_id
WHERE u.id = 'unit-commonwealth-2nd-bays'
ORDER BY r.position, s.stat_id;

SELECT * FROM unit_token WHERE unit_id = 'unit-commonwealth-2nd-bays';
SELECT * FROM generated_art WHERE unit_id = 'unit-commonwealth-2nd-bays';
```

The first correction patch carries five existing incorporated-errata findings
covering 14 cells, without altering their original observations. The Pz III E BAR
zero remains conditional on the documented date; it is not an unconditional
engine value. A corrected NULL intentionally overrides a non-NULL observation.

## Coverage remains partial

The initial DB contains 2,468 GPIDs / 1,307 distinct VASSAL labels, 1,356 canonical
candidates, 18 redirects, 1,450 manual entries and 124 profiles, 53 unit types,
21 stat definitions, 262 extracted documents and 61 generated-art sets. Only
63 candidates link to 118 GPIDs; 1,293 candidates and 2,350 slots remain unlinked.
There are 1,962 token filenames, including 14 missing source images. Original
manual comparisons remain limited to the previously reported 124 CPA comparisons,
12 organization rows and five errata; importing SQLite does not verify the rest.

## Distribution and frontend follow-up

**Do not copy this database to `public/`/`dist/` or embed it in Wasm.** It contains
private source text and quotations. The importer rejects destinations inside
`public/` or `dist/`, including resolved symlink paths; this does not prevent
someone separately copying or embedding it. See [licensing.md](licensing.md). Token records
retain `public_path = NULL`; private review paths are metadata for the existing
opt-in dev route, not publication permission. Generated art is never substituted
for an original source token.

Vite serves workspace files outside `public/` during development. Its always-on
`server.fs.deny` rule blocks `catalog.sqlite` and its journal/WAL/SHM sidecars,
including direct, `/@fs/` and raw/URL transform requests, whether private review
is enabled or disabled. The normal Vite sensitive-file deny rules are retained.
Encoded separators that are not file routes return 404 for binary requests;
HTML navigation can receive the SPA page, not database contents. This is not
authentication or a general sandbox for the repository. Vite's deny rule does
not filter `public/`, so the no-copy/no-embed boundary above remains essential.

No SQLite browser loader or HTTP endpoint is implemented here. The current UI
still embeds catalog/inventory JSON at Wasm compile time. Classification changes
and reviewed profile links reach it only after their bounded projections, sidecar regeneration and Wasm rebuild;
other SQL corrections still do **not** change that UI. A broader consumer migration
must explicitly project allowed columns/rows, omit private text and local
source-token routes, preserve IDs/redirects/order/null semantics, then change the
async loading boundary and retire obsolete JSON artifacts. Do not use `SELECT *`
to produce that projection. Public release requires per-class
rights review independent of source integrity. Scenario state remains ID-only.

## Tests

```sh
# Fast: stdlib + committed DB/compatibility JSON; no original artwork, Cargo or network
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p 'test_catalog_db.py'
# Explicit hydrated-source rebuild check; no replacement
python3 scripts/import-catalog-db.py --check
```

The fast suite covers patch ordering/once-only application, changed/deleted/earlier
history, failure atomicity/retry, re-import preservation, source-write rejection,
missing linked GPIDs, redirect integrity, unknown versus zero, nullable
corrections, token/generated-art separation, private-output rejection, and the committed DB read contract.
It also verifies classification-patch scope/failure/rerun behavior, type-only
projection with unknown/zero preservation and private-field exclusion, and
committed DB/catalog/sidecar type parity. The first command is included in
`test:functional` and the second in `test:source-data`, retaining `test_unit_data.py`.
Profile tests cover patch preconditions/atomic failure/rerun, unchanged numeric
observations/identity/art, explicit chart-code examples, linked numeric coverage,
catalog/sidecar parity, and projection field bounds, unknowns and private-text exclusion.
`tests/private-review.test.mjs` exercises HTTP denial with both opt-in states,
encoded/query forms and SQLite sidecars, while checking safe assets and review routes.
Run `npm run check`, `npm run test:e2e` and, with hydrated sources, `npm run test:sources`.
