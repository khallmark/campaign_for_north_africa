# Deterministic local tile pyramid

Run `npm run tiles` to generate ignored `public/tiles/map/manifest.json` and
`{z}/{x}_{y}.png`. This does not grant permission to distribute the source art.
While permission is pending, a default production build **must fail** if these
tiles are present. For work that does not need Vite to serve them, use
`npm run tiles -- --output /tmp/cna-map-tiles` instead.

The highest z is the original 14310×4632 RGBA image; z=0 fits a single 512px tile.
Each preceding level halves the original dimensions, rounding each axis up.
Every level is resized directly from the source with Lanczos3. A level's exact
board-to-raster scale is `width/board_width` and `height/board_height`; rounding
means these can differ slightly from `1/downsample`. Place tiles using their
level pixel origins (`x*512`, `y*512`) and these exact scales. Boundary tiles
retain their remaining width/height: no stretching or transparent padding.

The manifest records the source SHA256, generator/settings identifier, board and
level dimensions, downsample divisors, tile coordinates, pixel sizes, byte sizes
and SHA256s. Tile order is z, then y, then x. PNG uses fixed Best compression and
Adaptive filtering from locked image 0.25.10. No timestamps or absolute source
paths enter the manifest. Encoder/resampler changes must bump the generator ID.

Repeated runs verify source/settings/geometry and every tile hash before skipping
writes. Corrupt or missing tiles under a completed manifest cause regeneration.
The manifest is written last. If the first generation is interrupted before a
manifest exists, use a fresh empty output directory; unrecognized nonempty output
is never deleted. A board-size change likewise requires an empty directory,
avoiding stale tiles and preserving unrelated files. No parallel-writer guarantee
is provided; run only one generator per output directory.

`cargo test --locked -p cna-map tiles` uses a generated asymmetric 515×519 fixture
to check exact pixel reconstruction, tiny boundary tiles, deterministic separate
runs, unchanged-file reuse, corruption recovery and input/output safety.

## Browser research viewport

Expand **Full-board map · research** below the sandbox. This is a separate root
outside `#app`; sandbox turns and dialog renders do not replace its camera or
listeners. The shared startup entry calls `map::mount_research()` once. Map code
and its styles own the surface; sandbox units and movement are unchanged.

The viewer requests `/tiles/map/manifest.json` only when first opened. A missing,
invalid or timed-out manifest leaves an explicit artwork-unavailable placeholder,
with pan/zoom still available against census board dimensions. No source tiles
are bundled while permission is pending. The viewer does not grant permission,
change the distribution record or bypass the production guard.

Drag/pinch, cursor-anchored wheel zoom, arrows, +/− and Home operate in CSS pixels
through the shared viewport transform. Zoom runs from board fit to 4 CSS pixels
per board pixel. Exact per-axis level ratios position cropped boundary tiles.
Only the visible rectangle plus one tile ring is requested; a coarser level is
used if the DPR-aware candidate exceeds 100 live tile images. Old levels are
removed before new requests, and closing the viewer removes its tile nodes.
One additional level-zero image, when that level fits a single tile, stays behind
the detail ring to show the whole board while detail loads. Failed detail images
still retain labeled placeholders instead of silently presenting the preview as
full detail.

With private review enabled, the HTML loading screen also requests that preview
before WebAssembly downloads. Without opt-in, it shows only a local loading
status; production HTML contains no source preview request. External fonts load
without blocking the first paint, using system fonts until they are available.

The opt-in private dev server permits browser caching of map tiles and the
manifest, with mandatory content-hash revalidation on every reuse. Unchanged
bytes return 304; changed files return fresh bytes, and missing files return an
uncacheable error. Source-token responses remain `private, no-store`. This does
not enable shared caching, production serving, or source redistribution.
See [loading measurements](performance.md#private-review-loading-measurements).
Browser GPU retention and frame budgets still require measurement; a node cap
is not a memory proof.

```sh
npm run test:browser -- tests/browser/map-viewport.spec.ts tests/browser/smoke.spec.ts
```

Tests intercept the fixed tile URLs with explicitly synthetic images. They check
a 500px drag, three cursor-anchored zoom steps, request bounds, keyboard clamping,
two-finger pinch (Chromium touch emulation, not a physical device), missing tiles,
resize, close/reopen and sandbox-render isolation at both named viewports/DPRs.
Synthetic overview/gameplay captures establish layout, not historical map seams
or source-art permission. Source imagery must not be added merely to make a
portal screenshot look complete.
