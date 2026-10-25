# July 2021 text index: scope and unresolved evidence

CNA-166 indexes the checked-in text layer, not the complete game or all marks on
every page. [Coverage](coverage.md) is generated from the raw extracts and the
explicit [label aliases](extracts/label-aliases.json). No ledger rows, VASSAL
comparisons, fixture status, or claims of implementation are generated.

## Reproduce and check

```sh
cargo run --locked -p cna-sim --bin rules-ledger -- --rebuild-index --check-index
cargo run --locked -p cna-sim --bin rules-ledger -- --check-index
cargo test --locked -p cna-sim --test rules_index
```

The first command writes only `data/rules/case-index.json` and
`docs/rules/coverage.md`. The second is read-only and checks both outputs against
regeneration, extract hashes, PDF fingerprints against `docs/sources.lock.json`,
page counts, and alias matches. Neither requires Poppler or original PDFs: the
byte-exact extracts and source-lock metadata are versioned inputs. Full PDF-byte
verification belongs to `npm run check:sources` in a hydrated source checkout.

To reproduce the extracts, hydrate the PDFs with `git lfs pull`, run the full
source check, and use **Poppler pdftotext 22.12.0**:

```sh
pdftotext -raw -enc UTF-8 docs/CNAv2.1.0/Rules_Land_07-2021.pdf docs/rules/extracts/land-2021.txt
pdftotext -raw -enc UTF-8 'docs/CNAv2.1.0/Rules_Air&Logistics_07-2021.pdf' docs/rules/extracts/airlog-2021.txt
pdftotext -raw -enc UTF-8 docs/CNAv2.1.0/Rules_Scenarios_07-2021.pdf docs/rules/extracts/scenarios-2021.txt
cargo run --locked -p cna-sim --bin rules-ledger -- --check-index
```

Do not reflow text, remove form feeds, OCR over the text layer, or update hashes
merely to make a failed check pass. Extractor upgrades need a reviewed source
diff. `extracts/sources.json` records the command, version and both hashes.

## Labels are source identifiers, not floating-point numbers

The scanner recognizes exact `[section.case]` tokens, including a single lowercase
letter suffix (`4.44b`). Sections and cases retain their decimal strings; labels
are sorted lexicographically for stable JSON, not interpreted as real numbers.
Physical pages are determined by form-feed boundaries, including empty pages.
Unbracketed contents entries are excluded so they cannot supply false body-page
citations. Repeated labels use their first physical page, with every occurrence
listed separately in coverage (including `END OF CASE [59.0]`).

Thirty source-specific aliases recover malformed delimiters, spaces, or glyphs.
Each must match exactly once on its recorded page; each indexed row preserves
`raw_label`. They are indexing decisions, not changes to source text or rules.
The affected physical pages were reviewed: L21 10/14/35/42/44, AL21 5/7/23/25,
and SC21 5/7/8/11. Important exceptions:

- **AL21 p5 actually prints `[31.53]` beneath §34.5 Transports**, following
  `[34.52]` and before `[34.6]`. The index key is `34.53`, with raw `[31.53]`
  retained. This is a printed numbering anomaly, not an OCR repair or a rule
  interpretation. Original `docs/rules.txt` line 14182 also labels this text
  `[34.53]`. Do not cite an unqualified “AL21 §34.53” without this note.
- **L21 p10 prints `4.44b` twice**, once for the Z506 range addition and once
  for OA/reinforcement corrections. Both occurrences remain in the extract and
  coverage; one ledger label must account for both passages. `4.43a)]`, `4.44a`
  and `4.44c` remain distinct labels rather than disappearing into `4.44`.
- **SC21 p8 repeats `62.31`** for the deployment heading and an omission.
- **AL21 p25 runs `[55.23]` into the preceding sentence**. Indexing only labels
  at line starts would lose it, and would also lose `(addition:) [3.6]` in L21.
- SC21 `61.71`/`61.72` and AL21 `56.1` have unreliable extracted delimiter or
  digit glyphs. The raw spellings remain visible in the alias table.

## Why the count is not 1,634

The original planning count is reproducible with `pdftotext -layout`: **1,634**
distinct exact numeric bracket labels. The requested `-raw` extraction yields
**1,635**, adding SC21 `61.7` (physical p7); layout inserts whitespace into that
label. Neither count covers malformed labels or letter suffixes.

The index has **1,666** distinct labels: L21 **983**, AL21 **593**, SC21 **90**.
The additional 31 labels comprise three exact letter-suffixed labels and 28
recovered labels. The remaining two aliases only rename existing literal keys
or preserve already-counted source identity. Counts are observations, not fixed
targets; future image transcription will expand them.

## Image-only and mixed-page follow-up remains mandatory

SC21 physical **6, 10, 13–16** have empty raw text. Pages **9 and 12** contain
only a single correction overlay (`62.41` and `63.41` respectively), not their
underlying scenario text. Page **18** contains only its page number. These pages
must not be declared complete merely because the extractor exits successfully.
The missing/partial scenario cases affect §§61–64; §§64 and 65 currently have
zero indexed labels. Section 65 material and designer notes also need page-level
review rather than inventing an index entry from the contents page.

L21 **8, 9, 48** and AL21 **28** likewise yield only page numbers. This is a
text-layer gap list, not the full chart/image inventory: mixed pages can contain
unextracted images alongside abundant text. CNA-167 owns the exhaustive inventory
and E2.I8 owns scenario transcription. The index must be expanded through reviewed
transcription before official coverage/certification; zero counts never mean
`not_applicable`. VASSAL is comparison evidence, not a rules oracle.
