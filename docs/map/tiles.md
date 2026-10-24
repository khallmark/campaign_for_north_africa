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
