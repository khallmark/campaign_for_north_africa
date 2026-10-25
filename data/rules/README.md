# Rules coverage inputs

`ledger.schema.json` describes ledger row shape. The read-only `rules-ledger`
checker additionally rejects blank references, duplicate/unknown cases, mismatched
source locations, missing rows and insufficient statuses. It does not establish
that citations or a `verified` claim are true; source comparison and executed
fixtures remain mandatory. VASSAL is comparison evidence, never a rules oracle.

```sh
cargo run --locked -p cna-sim --bin rules-ledger -- --help
cargo run --locked -p cna-sim --bin rules-ledger -- --rebuild-index --check-index
cargo run --locked -p cna-sim --bin rules-ledger -- --check --section 8 --require verified
cargo run --locked -p cna-sim --bin rules-ledger -- --check --require verified,not_applicable --check-pages
```

Input availability:

- CNA-166: `case-index.json`, an array of `{case, section, book, page, raw_label?}`.
  Case keys are strings, not floating-point numbers. Page is the one-based PDF
  page, not printed pagination. Books are `L21` (sections 1–32, 48 pages), `AL21`
  (33–58, 28 pages) and `SC21` (59–65, 18 pages). Checked-in raw text, aliases and
  source fingerprints regenerate 1,666 indexed labels. See
  [coverage](../../docs/rules/coverage.md) and
  [source anomalies and image gaps](../../docs/rules/index-policy.md).
- Compare leaves: `ledger.json`, one schema-valid row per indexed case; not yet
  supplied. Index generation does not create placeholder comparisons.
- CNA-167 (not yet supplied): `evidence-index.json`, an object with `image_only_pages`, an array of
  `{book, page, status, transcript?}`. Status is `pending` or `transcribed`;
  transcribed records require an existing repository-relative transcript file.
  `--check-pages` reports pending pages without pretending they were transcribed.
  It validates the supplied inventory, not independent PDF image coverage.

`--data-dir PATH` selects another input directory for fixtures. Transcript paths
still resolve from the repository root. No checker output includes save content
or credentials. Checks are read-only; only `--rebuild-index` writes the repository
case index and coverage table (and disallows `--data-dir`). `--check-index` checks
extract hashes, PDF fingerprints against the source lock, and byte-identical
regeneration without requiring original PDFs. Full source-byte verification is
separate. Exit 1 indicates missing/invalid inputs or failed checks, not an
invitation to generate placeholder evidence.

Ordinary statuses are ordered `unread < compared < fixtures < implemented <
verified`; `--require` accepts any listed ordinary minimum. `not_applicable`
requires a nonblank justification and explicit inclusion in the requirement
list. A section filter limits completeness/status checks, but malformed or
unknown rows elsewhere still fail. Neither an empty index nor an unindexed
requested section can pass vacuously. Official certification also requires
reviewed source conflicts, detailed logistics, and historical disclosure.
