# Source-image grid calibration

Source: `docs/CNAv2.1.0/images/CNA Map Vassal Mitch Guthrie 2021.png`, SHA256
`e82f995e213da486004d006e5b7fd1f6e49e7464fb7fd2db23f36bbbc57b14b4`.
Thirty measured anchors cover Malta and A–E, five per section. This is an
image-grid alignment check, not reviewed terrain, coast, seam or scenario data.

## What the image can establish

The inspected image crops show printed grid lines, towns and symbols but no
printed hex IDs. Hex numbers therefore remain derived from the VASSAL numbering
definitions, not OCR of absent labels. Recognizable anchors include Benina,
Soluch, Derna, Mechili, Buq Buq, Bardia, Tobruk, Giarabub, Matruh and Aboukir.
The ticket's suggested name “B4827 Benghazi” is not corroborated by that crop;
it is retained as a **grid-only** anchor, not a city assignment. No terrain or
scenario label is inferred from this calibration. Scenario identity reconciliation
and ambiguities remain separate work.

## Measurement method and uncertainty

Measurements were made against the original RGB image, not a rendered overlay.
For each raw census center, sample 29 evenly spaced positions on the middle
15–85% of each of the six Voronoi edges. Search translations from −5 to +5 board
pixels on each axis in 0.25px steps. Bilinearly sample the source's Pillow `L`
luminance at those 174 positions. Minimize the mean luminance clipped to [60,200],
which locates dark grid strokes while reducing interference from black symbols
and light backgrounds. Equal scores use y-then-x search order. The observed
center is the raw center plus that best image-derived translation. Fixed observed
coordinates are recorded in `calibration::ANCHORS`; tests do not generate them
from the implementation being tested.

All crops must also be visually inspected: dark artwork could fool the numerical
fit. Two initial sea-only candidates (A3020 and D4820), which lacked grid strokes,
were rejected rather than assigned zero residuals. The final A3028/A2028 and
D3010 candidates have visible grid strokes. The fitted lattice uses continuous
Voronoi edges, not an assertion that the raster strokes are mathematically exact.
Quarter-pixel search quantization alone is about ±0.18px radially; line thickness,
antialiasing, resampling and artwork interference add unquantified uncertainty.
The listed residuals are alignment estimates, not survey-precision coordinates.

## Correction table

Use the component-wise median of the five observed translations in each section.
Translations apply to the grid origin before both forward geometry and inverse
hit testing. They leave spacing, shared-edge normals and all source JSON intact.
They are stored only in `crates/cna-map/src/calibration.rs`, never patched into
`public/data/map/sources.json`.

| Section | Δx | Δy |
| --- | ---: | ---: |
| Malta | +0.25 | 0 |
| A | +1.25 | +2.25 |
| B | +2.00 | +0.50 |
| C | +2.25 | 0 |
| D | −0.75 | −0.25 |
| E | 0 | +0.50 |

## Anchor residuals and inspected crops

`map-census --anchors` writes each filename below under
`.amp/in/artifacts/map-calibration/`, plus `anchors.json` with observed and computed
centers and full-precision residuals. Crops show a magenta computed outline and a
cyan observed-center cross, enlarged 2× with nearest-neighbour display scaling.
Distances below remain **board pixels**, not enlarged display pixels.

| Crop | Before | After |
| --- | ---: | ---: |
| Malta-0602.png | 0.250 | 0.354 |
| Malta-1003.png | 0.000 | 0.250 |
| Malta-0805.png | 0.354 | 0.250 |
| Malta-0906.png | 0.250 | 0.000 |
| Malta-1204.png | 0.250 | 0.000 |
| A4829.png | 2.795 | 0.250 |
| A4130.png | 2.704 | 0.250 |
| A4929.png | 2.462 | 0.250 |
| A3028.png | 1.953 | 0.750 |
| A2028.png | 1.601 | 1.250 |
| B4827.png | 3.041 | 1.000 |
| B5925.png | 2.926 | 0.901 |
| B4921.png | 2.062 | 0.000 |
| B4020.png | 2.016 | 0.250 |
| B3020.png | 2.062 | 1.000 |
| C3926.png | 3.250 | 1.000 |
| C4321.png | 2.761 | 0.559 |
| C4807.png | 1.346 | 1.118 |
| C1014.png | 2.500 | 1.521 |
| C3617.png | 2.264 | 0.250 |
| D3714.png | 0.750 | 0.250 |
| D3612.png | 0.791 | 0.000 |
| D3615.png | 0.559 | 0.250 |
| D2020.png | 0.791 | 1.118 |
| D3010.png | 1.346 | 0.559 |
| E3815.png | 1.000 | 0.500 |
| E1829.png | 1.500 | 1.581 |
| E1430.png | 1.275 | 1.458 |
| E2015.png | 0.250 | 0.559 |
| E3015.png | 0.559 | 0.250 |

Maximum measured residual falls from 3.250px to 1.581px. All sampled anchors are
within 3px; this is not proof that every unsampled cell is within that tolerance.
Residual drift within sections may warrant a larger independently reviewed sample
later. No scale/rotation fit or undocumented per-hex adjustment is applied.

```sh
cargo run --locked -p cna-map --bin map-census -- --anchors
cargo test --locked -p cna-map calibration
```

The crop command refuses a different source checksum. Source changes require new
measurements and review rather than silently reusing these observations.
