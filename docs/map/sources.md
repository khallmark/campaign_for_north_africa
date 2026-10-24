# Map source census

`public/data/map/sources.json` inventories the **Main Map / CNA Original** board
in `docs/CNAv2.1.0/buildFile.xml`. It is source evidence, not a terrain database,
calibrated geometry contract, or permission to redistribute artwork.

## Reproduce and check

```sh
cargo run --locked -p cna-map --bin map-census -- --write
cargo run --locked -p cna-map --bin map-census -- --check
cargo test --locked -p cna-map census
```

The Rust tool uses the build-time repository root, so these paths do not depend
on the invoking directory. No arguments defaults to read-only `--check`.
`--write` regenerates the JSON deterministically; `--check` exits nonzero on
missing or stale output. Review source changes before regenerating. The tool
hashes the full PNG as a stream and reads its IHDR dimensions without decoding
the bitmap. It does not copy artwork or generate tiles.

## Provenance

- Source revision: `5b8f99288c2f1c6152397b366462373a58050b3f` in
  `khallmark/campaign_for_north_africa`.
- XML: `docs/CNAv2.1.0/buildFile.xml`, SHA-256
  `944c4f7f80e505093d8001453cd7e4c1b57064206d768724e3889b6b5f696ac6`.
- Board image: `docs/CNAv2.1.0/images/CNA Map Vassal Mitch Guthrie 2021.png`,
  **14310 × 4632 pixels**, SHA-256
  `e82f995e213da486004d006e5b7fd1f6e49e7464fb7fd2db23f36bbbc57b14b4`.
- The module identifies itself as CNA 2.1.0, VASSAL 3.7.15. Its root description
  credits `(c) 1979 SPI/(c) 2022 White Box Games`; this is not a license grant.

Grid attributes and numbering attributes are retained as XML-decoded **strings**
under `grid.attributes` and `grid.numbering`, preserving names, signs, decimal
notation and flags rather than silently interpreting VASSAL coordinate rules.
Polygons retain source vertex order as integer board-pixel pairs. Zone order is
XML order; attribute maps are sorted for reproducibility. Exact source names are
retained, including the doubled space in `Alexandria  Holding Box Hex 3714`.

## Six grids, 33 zones

All six grids use `dx=72.95`, `dy=85.25`, `sideways=true`, `vDescend=true`.
Malta is separate from the A–E battlefield sections.

| Zone | x0 | y0 | hOff | vOff | stagger | Estimated cells |
| --- | ---: | ---: | ---: | ---: | --- | ---: |
| Malta | -15 | 4 | 6 | -1 | false | 71 |
| Map A | 55 | 48 | 6 | 0 | true | 1001 |
| Map B | -16 | 7 | 2 | -33 | false | 1974 |
| Map C | -16 | 10 | 12 | -66 | false | 1534 |
| Map D | -16 | 16 | 21 | -99 | false | 1254 |
| Map E | -17 | -66 | 19 | -133 | false | 1306 |

The estimate is absolute polygon area (shoelace formula) divided by `dx × dy`,
rounded to the nearest integer. The total **7140** includes sea and boundary
fractions. It is not a count of playable hexes, enumerated centers, unique seam
cells, or reviewed terrain. Later calibration, seam reconciliation and terrain
transcription own those questions; unknown terrain must never default to clear.

There are **27 non-hex zones**, not the planning estimate of "30+ non-hex zones":

- **14 `off_map_box`:** Sicily, Italy, Benghazi, Derna, Crete, Bardia, Tobruk,
  both Alexandria holding boxes; Tunis, Gabes, Tripoli, Tripolitania and German
  Shipping.
- **4 `track`:** Tunis–Gabes, Gabes–Tripoli, Tripoli–Tripolitania and
  Tripolitania–Nofilia.
- **9 `ui_track`:** Weather Track, Map A–E Weather, Map D/E Operation Stage and
  Game Track.

Classification is a source-inventory label only; it does not imply movement
adjacency or gameplay effects. An unknown non-hex zone fails generation until
reviewed, rather than silently assigning it to a default class. Square-grid
attributes and their numbering (when present) are retained too. Other maps in
the module, counters, saved-game/player records and terrain are not imported.

The census tests compare every Main Map zone's polygon and every grid/numbering
attribute against parsed XML, assert the six expected sections and category
counts, and run the read-only source check. The independently measured image
checksum and asymmetric Map A/E parameters guard against partial hashing and
accidental reuse of one section's offsets.

## Seam identity conflict (CNA-20, unresolved)

The July 2021 Land Rules §4.1, printed page 7, says the **01xx hex-row** of
each section overlays the **39xx hex-row** of its predecessor. The rendered PDF
was inspected; this is not an OCR inference. Its SHA-256 is
`78256d7dd4327216d75aaae0ce8b6980c052ceb81e04706bf1036f6d6546e270`.
The original `docs/rules.txt` §4.1 agrees, with OCR spelling `0ixx`.
The planning ticket's `xx01`/`xx39` paraphrase reverses the digit pairs and must
not be used to generate a seam table.

Neither that paraphrase nor a literal 39xx/01xx identification matches the
current VASSAL numbering. Applying the documented census numbering and current
calibration translations gives these **candidate co-located centers**, in board
pixels, at the four section boundaries:

| West identity / center | East identity / center | Separation |
| --- | --- | ---: |
| A3033 / (2862.500, 2391.650) | B3000 / (2864.875, 2391.850) | 2.383px |
| B3033 / (5678.125, 2391.850) | C3000 / (5681.375, 2391.350) | 3.288px |
| C3033 / (8494.625, 2391.350) | D3000 / (8497.625, 2391.100) | 3.010px |
| D3033 / (11310.875, 2391.100) | E3000 / (11314.875, 2390.850) | 4.008px |

These values follow from `max + hOff - col`, section `vOff`, the stagger flag,
and the translations in [calibration](calibration.md); they are not independent
survey measurements or approved printed aliases. As a counterexample, A3930 is
at (2564.125, 1735.100), while B0130 is at (5379.750, 4507.400). A literal
39xx/01xx alias would merge distant cells under the current identity contract.

The calibrated image has no printed hex labels in the inspected anchor crops.
Four additional 360×300px seam crops centered near y=2391, starting at x=2682,
5498, 8314 and 11130, were inspected at 2× scale. They also have no readable
numeric hex labels. Private review captures are
`.amp/in/artifacts/map-seams/{AB,BC,CD,DE}.png`; these are source-derived research
artifacts, never public portal assets. This sample is not a search of all scans.
Image alignment alone cannot establish a conversion from these VASSAL names to
the rulebook's printed names.

The source-owner adjudication preserves existing section-local `HexId` and
occupancy semantics. Separately versioned, source-hash-pinned VASSAL seam
candidates may record residuals with original-rule numbering explicitly
unresolved, but must not become official equivalences, deduplicate occupancy or
enable official movement. CNA-20's official/canonical seam acceptance remains
blocked on reviewed bridging evidence, not on permission to build a VASSAL-only
candidate dataset. No guessed alias, occupancy merge or cross-section adjacency
has been added. Malta remains separate; no land connection to A–E is implied.
