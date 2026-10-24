# Hex identity and adjacency conventions

`cna_map::hex` owns section-local VASSAL-derived identities, not playable-map
membership or verified original printed identities. `HexId` stores section plus
the first (`col`) and second (`row`) digit pairs in the module's numbering.
The parser accepts exactly `A`–`E` plus four ASCII digits, or `Malta NNNN`.
`Malta 0805` is the module's Royal Navy send-to location. There is no guessed `M`
prefix, OCR correction, whitespace trimming, lowercase alias, or `Map C` alias.
Coordinates 00–99 are syntactically representable; successful parsing does not
prove a cell exists inside a section polygon, is land, or permits movement.

No reviewed mapping establishes equivalence to the original rulebook's printed
hex numbers. Image calibration establishes grid alignment, not that identity
bridge. See the [seam evidence conflict](sources.md#seam-identity-conflict-cna-20-unresolved).
Section-local IDs and occupancy remain distinct across sections; apparent image
co-location must not merge them or enable official cross-section movement.

`MapLocation` distinguishes hexes from the 14 exact named `OffMapBox` values in
[the census](sources.md). Names preserve source spelling, including the doubled
space in `Alexandria  Holding Box Hex 3714`. Tracks and weather/UI zones are not
hexes or off-map boxes. Boxes have no hex-neighbour operation.

## Numbering evidence

The source module uses VASSAL 3.7.15. Its relevant algorithms are
[HexGridNumbering](https://github.com/vassalengine/vassal/blob/3.7.15/vassal-app/src/main/java/VASSAL/build/module/map/boardPicker/board/mapgrid/HexGridNumbering.java)
(`getRow`, `getColumn`, `getMaxRows`),
[RegularGridNumbering](https://github.com/vassalengine/vassal/blob/3.7.15/vassal-app/src/main/java/VASSAL/build/module/map/boardPicker/board/mapgrid/RegularGridNumbering.java)
(`getName`), and
[Zone](https://github.com/vassalengine/vassal/blob/3.7.15/vassal-app/src/main/java/VASSAL/build/module/map/boardPicker/board/mapgrid/Zone.java)
(`getSize`: polygon bounding-box size). These are numbering evidence, not a rules
oracle. The implementation does not copy their pixel rounding or Java code.

For these sideways grids, raw grid column `q` runs down the image; raw row `r`
runs right. The source centers alternate half a spacing to the right when `q`
is odd. Let `max = floor(zone_height / dx + 0.5)` and `p = q mod 2` (0 or 1).
All six have `vDescend=true`, `hDescend=false`, `first=H`:

```text
VASSAL col = max - q + hOff
VASSAL row = r + vOff + (stagger ? p : 0)
```

| Section | max | hOff | max + hOff | stagger |
| --- | ---: | ---: | ---: | --- |
| Malta | 9 | 6 | 15 | false |
| A | 56 | 6 | 62 | true |
| B | 61 | 2 | 63 | false |
| C | 51 | 12 | 63 | false |
| D | 42 | 21 | 63 | false |
| E | 44 | 19 | 63 | false |

Thus **all six sections have odd VASSAL columns half a row west of even
columns**, despite Map A's different flags. Increasing VASSAL `col` moves
north; increasing VASSAL `row` moves east. This is not the sandbox simulation's
coordinate system. Tests derive the result independently from each census grid
and polygon; changing a source offset cannot silently leave the parity assumption
unnoticed.

`Direction::ALL` is clockwise in image coordinates: East, SouthEast, SouthWest,
West, NorthWest, NorthEast. Direction zero is the east-facing edge. For an even
VASSAL column, SE is `(col−1, row+1)` and SW is `(col−1, row)`; for an odd column,
SE is `(col−1, row)` and SW is `(col−1, row−1)`. Northward diagonals increment the
column with the same row changes. East/west increment/decrement only the row.

`HexId::neighbor` returns section-local **lattice** neighbours. It returns `None`
at the 00/99 identity limits, not at coastline or polygon boundaries. It does not
claim passability. The seam table is deliberately empty until E3.I1.S2 reviews
bridging evidence between original printed and VASSAL identities. Separately
versioned, source-hash-pinned VASSAL seam candidates may record residuals, but
remain research observations with original-rule numbering unresolved. They do
not satisfy CNA-20's official/canonical seam acceptance, deduplicate occupancy,
or authorize official movement. Consumers must not infer A↔B connections or
routes to holding boxes. `HexsideId::new` canonicalizes the two endpoints to the lower
`(section, col, row)` hex plus its direction toward the other; both endpoints
produce exactly the same side identity.

## Verification and remaining contract work

```sh
cargo test --locked -p cna-map hex
cargo clippy --workspace --all-targets -- -D warnings
```

The finite-domain property test exhausts all 60,000 printable identities for
parse/display round trips, symmetric neighbours, six distinct interior
neighbours, and canonical shared sides. An independent source-numbering test
checks the direction of every interior step, so a consistently mirrored graph
cannot pass on symmetry alone. Scenario examples include C4218, C4120, C3617,
D3714, D3612, E3815, B5925 and A4829. These are identity fixtures, not proof of
terrain or scenario transcription. The published consumer fixture, serialization
schema, pixel geometry and `map-v1` contract remain later E1.I2 leaves.
