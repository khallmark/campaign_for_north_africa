# Tile memory and transfer budgets

Measured with the locked PNG generator from CNA-61 and source SHA256
`e82f995e213da486004d006e5b7fd1f6e49e7464fb7fd2db23f36bbbc57b14b4`.
The generated manifest SHA256 is
`9bb1421f9e1a41f68c9b68a299dfa6b488fd3fd7357877593b6819d60c908b23`.
These are offline measurements, not browser performance results. Source-derived
tiles remain private research output; distribution permission is still pending.

## Encoded pyramid

| z | Raster size | Columns × rows | Tiles | PNG bytes |
| --- | --- | --- | ---: | ---: |
| 0 | 448 × 145 | 1 × 1 | 1 | 112,215 |
| 1 | 895 × 290 | 2 × 1 | 2 | 396,728 |
| 2 | 1789 × 579 | 4 × 2 | 8 | 1,287,889 |
| 3 | 3578 × 1158 | 7 × 3 | 21 | 3,839,013 |
| 4 | 7155 × 2316 | 14 × 5 | 70 | 10,701,619 |
| 5 | 14310 × 4632 | 28 × 10 | 280 | 17,182,191 |
| Total | | | 382 | 33,519,655 |

The manifest adds 85,020 bytes, for **33,604,675 encoded bytes** in the complete
pyramid (about 32.05 MiB). This is not the initial viewport payload. A single full
RGBA source decode costs **265,135,680 bytes** (252.85 MiB); keeping the full
image decoded is not the viewport strategy. A full 512 × 512 RGBA tile costs
1 MiB; boundary tiles cost less. PNG byte sizes do not predict decoded memory.

<!-- Machine-readable baseline, used directly by the manifest regression test. -->
Tile byte budget: 33604675

The budget includes every PNG plus the manifest. The regression allows growth
through 110% of that baseline: **36,965,142 bytes** (integer floor); one more byte
fails. Changing compression or source data requires reviewing these measurements,
not silently increasing the budget.

## Expected visible tiles before browser work

The following conservative upper bounds treat the entire named viewport as map
content, rather than subtracting interface chrome. Overview fits board width;
gameplay means 1 CSS pixel per board pixel. Choose the first level with both
raster/board axis ratios at least zoom × DPR, capped at the original level.
For each axis use `ceil(viewport * raster/board / zoom / 512) + 1`, clipped to
the level's tile count; the extra tile covers arbitrary pan alignment. Add one
tile on each side for the request ring, then clip again to the board.

| CSS viewport | DPR | View | Level | Visible ≤ | With ring ≤ | RGBA ring ≤ MiB |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| 2458 × 1358 | 1 | Overview | 3 | 21 | 21 | 21 |
| 2458 × 1358 | 2 | Overview | 4 | 70 | 70 | 70 |
| 2458 × 1358 | 1 | Gameplay | 5 | 24 | 48 | 48 |
| 2458 × 1358 | 2 | Gameplay | 5 | 24 | 48 | 48 |
| 1280 × 800 | 1 | Overview | 2 | 8 | 8 | 8 |
| 1280 × 800 | 2 | Overview | 3 | 21 | 21 | 21 |
| 1280 × 800 | 1 | Gameplay | 5 | 12 | 30 | 30 |
| 1280 × 800 | 2 | Gameplay | 5 | 12 | 30 | 30 |

These bounds are not an all-zoom proof. Intermediate zooms and level transitions
can request more tiles, especially at DPR 2. The viewport must cap retained tiles
and may need a coarser level to meet the targets; do not keep two entire levels
alive during a transition. At gameplay zoom the original raster caps detail, so
DPR 2 does not double the tile count or invent higher-resolution source pixels.

## Initial browser targets — not measured claims

At both named viewports and DPR 1 and 2:

- Decoded visible/ring/cache tile data: ≤128 MiB, including transition retention.
- Live tile nodes: ≤100; requests confined to visible tiles plus one tile ring.
- Scripted-pan p95 frame interval: ≤33 ms. Measure after warm-up and report
  cold loading separately, with browser/device/zoom and retained cache state.

CNA-58 owns browser measurements and any evidence-backed target revision. A
passing offline byte budget does not establish frame time, GPU memory, browser
decode overhead, network latency or tile eviction behavior.

## Private review loading measurements

Chromium, 2458 × 1358 CSS pixels, DPR 2, actual private PNG tiles, with CDP
network emulation at 10 Mbps and 60 ms latency. Each run starts with a fresh
browser context, then reloads in that same context; no request interception.
Tile time runs from the first tile request to the last tile response. These
single-run measurements describe transfer/loading, not frame time or GPU memory.

| State | Before: tile transfer / time | After: tile transfer / time |
| --- | --- | --- |
| Cold | 8,987,043 bytes / 7,284 ms | 9,099,558 bytes / 7,386 ms |
| Reload | 8,987,043 bytes / 7,288 ms | 18,300 bytes / 698 ms |

Reload transfer fell 99.8% and tile time fell about 90%. The server revalidates
cached tiles and the manifest by content hash; changed or missing files do not
reuse stale responses. Private opt-in and distribution restrictions are unchanged.

Cold full-detail transfer is unchanged apart from one 112,215-byte overview.
That overview appeared after 560 ms cold (63 ms on reload), covering blank tile
placeholders while original-resolution detail arrived. It adds one 448 × 145
image (259,840 decoded RGBA bytes) outside the 100-detail-tile cap. This is not a
claim that cold full-resolution loading or frame time improved.

Startup no longer waits for WebAssembly to request the private preview, and
Google Fonts no longer blocks the application stylesheet. Separate cold captures
at 1280 × 800 and 390 × 800 (DPR 2, the same network emulation) painted content at
352/284 ms and finished the preview response at 388/390 ms, measured from
navigation. WebAssembly finished downloading at 4,440/4,329 ms. The workspace
becomes interactive after WebAssembly initializes; the earlier preview is only
loading feedback, not an interactive map. Both widths had no horizontal overflow.

## Reproduce privately

```sh
npm run tiles -- --output /tmp/cna-map-tiles
cargo test --locked -p cna-map tiles
cargo test --locked --release -p cna-map --test tiles -- --ignored --nocapture
```

The full-board budget test regenerates the actual source pyramid into a temporary
directory, sums fresh manifest entries, checks PNG sizes against those entries,
then removes it and enforces the limit. It is an explicit ignored test because
66-megapixel resampling takes about 35 seconds even in release mode, and much
longer in debug. Run the third command whenever the tile generator, encoder,
source or recorded budget changes; ordinary workspace tests retain the fast
synthetic tile tests. The test must not be reported as passed just because the
default suite skipped it. No source artwork is added to `public`, `dist`, or a
portal by this check. A temporary 30,000,000-byte baseline was independently
verified to fail against the freshly generated 33,604,675-byte pyramid.
