# Committed SQLite catalog

`data/catalog.sqlite` is the **committed private reference database**, not a build
cache. VASSAL is the import source; numbered SQL files record reviewed corrections
and small curated changes. Commit the importer/SQL changes and resulting database
together. The existing JSON files remain unchanged compatibility inputs for the
current frontend; they are **not read by ordinary database imports**.

## Import and correction workflow

Python 3.11+ and SQLite 3.37+ are required. Source import also needs Cargo and the
hydrated, integrity-locked source checkout. No Python packages are required.

```sh
python3 scripts/import-catalog-db.py              # VASSAL first, then pending SQL
python3 scripts/import-catalog-db.py --check      # same import, detect drift, never replace
python3 scripts/import-catalog-db.py --patch-only # SQL only; no sources/Cargo needed
```

The source command verifies `docs/sources.lock.json`, runs the existing Rust
`vassal-census` against `docs/CNAv2.1.0/buildFile.xml`, and loads its temporary
output. It does not reparse VASSAL independently or use the committed JSON census
as the import source. Images remain files: SQLite stores paths, hashes, existence,
layer labels and source locators, not image bytes or decoded save state.

Add `data/sql/patches/002-<description>.sql` (then 003, etc.). Filenames sort
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

No SQLite browser loader, HTTP endpoint or safe public projection is implemented
here. The current UI still embeds catalog/inventory JSON at Wasm compile time;
SQL corrections in this DB do **not** change that UI yet. A later coordinated
consumer migration must explicitly project allowed columns/rows, omit private
text and local source-token routes, preserve IDs/redirects/order/null semantics,
then change the async loading boundary and retire obsolete JSON artifacts. Do not
use `SELECT *` to produce that projection. Public release requires per-class
rights review independent of source integrity. Scenario state remains ID-only.

## Tests

```sh
# Fast: stdlib + committed DB; no original artwork, JSON inputs, Cargo or network
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p 'test_catalog_db.py'
# Explicit hydrated-source rebuild check; no replacement
python3 scripts/import-catalog-db.py --check
```

The fast suite covers patch ordering/once-only application, changed/deleted/earlier
history, failure atomicity/retry, re-import preservation, source-write rejection,
missing linked GPIDs, redirect integrity, unknown versus zero, nullable
corrections, token/generated-art separation, private-output rejection, and the committed DB read contract.
There is no public projection to certify yet. On infrastructure integration,
append the first command to `test:functional` and the second to `test:source-data`,
retaining `test_unit_data.py`; do not silently treat absent tests as success.
