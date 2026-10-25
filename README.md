# The Campaign for North Africa — Field Command

A full-board review workspace for SPI's _The Campaign for North Africa_: pan and zoom the supplied map, browse source records, arrange explicitly user-placed planning counters, and inspect the 111-Game-Turn calendar. The old fictional Tobruk teaching scenario is no longer mounted. An official simulation and historical deployments are not loaded.

## Run

Use Node.js 24.22.1 (pinned in `.node-version`) and its bundled npm, plus Rust 1.99.1 and the WebAssembly target (pinned in `rust-toolchain.toml`). Application state, simulation, DOM/SVG rendering, input handling, and sprite motion are authored in Rust. JavaScript is limited to generated Wasm bindings and their loader; Vite serves the browser assets.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.105 --locked
npm ci
npm run dev
npm test
npm run build
```

`npm run dev` builds Rust/Wasm before starting Vite, which prints the development URL. After editing Rust, run `npm run wasm` and reload the page. `npm run preview` serves the production build. No game account, backend, or API key is needed. Fonts use Google Fonts with local system fallbacks. Play is session-only: reloading starts again.

## Browser tests

```sh
npx playwright install --with-deps chromium  # Also run by .agents/setup
npm run test:browser -- tests/browser/smoke.spec.ts
```

The harness builds Rust/Wasm and serves a fresh production preview on port 4175,
then stops it when the run ends. Keep that port free; it never reuses an existing
server. Chromium runs at 2458×1358 and 1280×800, each at DPR 1 and 2.
To select one project, append `--project=chromium-1280x800-dpr1`.
Screenshot snapshots live under ignored `tests/browser/__screenshots__/`;
failure screenshots, videos and traces go to ignored `test-results/`.
The smoke suite checks floating controls, selection anchoring at viewport edges, delayed tooltips,
keyboard dialogs, reference calendar boundaries and narrow layout. The map suite
also checks missing artwork, tile budgets, cursor zoom and touch pinch.

## Amp orbs

Amp runs `.agents/setup` when preparing a fresh or stale project snapshot. It installs the pinned Node.js release only when needed, checks the official download checksum, runs `npm ci`, and prepares Rust, the Wasm target, and the matching wasm-bindgen CLI. Matching snapshots skip setup and retain the installed dependencies. The build script makes Rust's user-local tools available to supervised services.

```sh
.agents/setup             # Safe to rerun manually; installs tools and dependencies
amp orb services ensure   # Starts the supervised Vite server and prints its portal URL
```

The development service is declared in `.amp/services.yaml`; setup does not start it. No resume hook is needed because there are no credentials, tunnels, or databases to refresh. Generated `.amp/portals/` state stays untracked. Setup changes must reach the project's default branch before future orbs can use them.

### Authorized private full-map review

The main full-board map can serve source-derived tiles in an explicitly
authorized review orb without putting them in `public/` or `dist/`:

```sh
cargo run --locked --release -p cna-map --bin cna-map-tiles -- --output /tmp/cna-private-review-map
# Set CNA_PRIVATE_REVIEW_TILES=/tmp/cna-private-review-map in ignored .env.local.
npm run build
amp orb service restart cna
amp orb services ensure
```

The full board opens directly as the main map. The opt-in Vite
development middleware serves only the manifest and numbered PNG tiles at
`/tiles/map/`, with private/no-store responses. It is absent from production
builds and previews. Remove the local environment setting and restart to disable
it; regenerate the pyramid if the temporary directory is lost. Do not commit
the opt-in or copy the tiles into release assets.

The same opt-in serves private PNG/SVG token sources at
`/__source-tokens/<encoded basename>` from `docs/CNAv2.1.0/images/`.
It permits GET/HEAD only, rejects path traversal, sends private/no-store headers
and sandboxes standalone SVG content. These source images are distinct from
generated illustrations; their availability does not clear public redistribution.

Kevin authorized this source artwork for his existing review portal on
2026-10-24. That is not a third-party redistribution grant or public-release
authorization: `docs/map/distribution.json` remains pending, and the default
build still rejects uncleared tiles in public assets. Historical seams, terrain,
scenario identities and official rules remain unverified.

## Review board

- Drag, scroll or pinch the map; with it focused, arrow keys pan, + / − zoom, and Home fits the board.
- **Reinforcements** adds a candidate identity as a planning counter. Click a mapped position, or pan and press Enter to place at the viewport center. No historical position, terrain legality, supplies or combat value is inferred.
- Select a counter from the left order of battle or the map. Information floats on the right; icon actions stay below the counter. Hover tooltips appear after one second; keyboard focus shows them immediately. Reposition and remove affect only this review session. Combat, fortification, supply and End stage explain that no simulation is connected.
- The date chip opens all **111 weekly Game-Turns**, with three Operations Stages each. Labels follow the supplied printed track from September III, 1940 through January I, 1943, using four labelled weeks per month rather than Gregorian date arithmetic. The cursor is reference-only. Track events and reinforcement annotations are not an adjudicated schedule; the calendar links back to the original track on the map.
- **New operation** confirms clearing the review board, not loading a fictional scenario. All work remains session-only.

Canonical identity IDs are preserved. Displayed illustrations are generated artwork, not canonical game-token images. Missing artwork stays labelled. Section-local positions use the map geometry's VASSAL-derived IDs, not certified printed aliases; no cross-section adjacency or occupancy merging is inferred.

## Retained teaching model (not mounted)

The following behavior documents the preserved sandbox code and regression tests, not the full-board review UI:

- Select a Commonwealth formation on the map or in the order of battle.
- **Move:** choose a highlighted destination. Ground movement costs 1 CP in desert and 2 on ridges. Motorized units use 8 fuel per CP; foot troops and camel scouts do not. Ships stay at sea; aircraft cross land, sea, occupied hexes and ground control for 1 CP per hex, but cannot end on an occupied hex. Entering an armed enemy's adjacent hex in the same ground/naval domain ends movement.
- **Attack:** choose an eligible enemy in the selected role's range and confirm the preview. Costs 2 CP and 20 ammunition. A surviving defender returns fire only if it can engage the attacker at that range. Artillery can bombard; fighters intercept aircraft; bombers attack ground/naval targets; flying boats attack ships; AA can engage aircraft and ground units. Combat is deterministic.
- **Dig in:** spend 1 CP for 10 damage protection until moving or attacking.
- **Resupply:** spend 1 CP within 2 hexes of a stocked truck or the frontier depot. Trucks transfer up to 25 of each resource, limited by recipient capacity; the depot refills all stores. Trucks need at least 25 of each resource to act as a supply source.
- **End stage:** Axis formations move and attack; supply upkeep consumes water and vehicle fuel, then CP refreshes. Every third incoming stage is hot. Units at zero water cannot move/assault and lose strength at upkeep.
- **Reinforcements:** search the candidate-identity inventory or expand the fictional role roster, choose player or enemy control, then place on an empty highlighted hex. All 24 sandbox capability roles are available, including the additional twenty. These values do not come from historical catalog characteristics. AI combat units engage compatible targets; support units do not issue autonomous support orders.
- **Support:** engineers fortify adjacent ground combat units for 1 CP; headquarters spends 2 CP to restore 1 CP to a neighbor; recovery vehicles spend 1 CP and 20 fuel to repair 20 strength on a surviving adjacent ground vehicle. Ground support units transfer stores like supply trucks.
- Move a surviving ground combat formation into Tobruk to win. Aircraft, ships and unarmed support units cannot capture it. Lose after stage 12 or when all player ground combat formations are destroyed.

Map controls toggle the grid and geometric supply range. Zoom centers on the selected formation. All buttons, formations, and highlighted destinations support keyboard activation. Escape cancels the current order or closes a dialog.

**Unit catalog:** search 1,450 source records and 124 characteristic profiles by name, nationality, role, or ID, filter by record type and side, and expand a record for characteristics and source citations. Only individually sourced values are shown, preserving printed notation and conditional restrictions; verified identity does not imply verified stats. These are source rows, not 1,450 unique historical units or deployable formations. The coverage notice describes unfinished scenario data and counter reconciliation. See [catalog provenance and limitations](docs/unit-catalog.md).

**Unit identities:** `public/data/unit-identities.json` links provisional canonical identities to source rows, sandbox roles and individual portrait/atlas paths. Aircraft load configurations may share an identity, while formation aliases still need review; its candidate count is not a certified count of distinct historical units. Reinforcement, battlefield, portrait and catalog views use reviewed identity-specific artwork when available. Missing/candidate art retains an explicitly labeled shared-role placeholder and does not count as unique-art coverage. The original seven scenario formations remain prototype proxies, not falsely matched to headquarters records. Unique artwork for every identity is **not complete**.

## Scope and source material

The supplied rulebook text is in `docs/rules.txt`. This prototype adapts the ideas of operations stages (§5), capability points (§6), hex movement and enemy zones (§8), close assault (§15), fuel loss (§49), and water supply (§52). Three stages represent one week; hexes are nominally 8 km.

**This is not a rules-complete implementation or a historical scenario.** Forces, positions, terrain, timings within stages, values, weather cycle, and combat formulas are invented for a playable demonstration. Air, naval and support actions are simplified sandbox behaviors, not the rulebook's full systems. The simulation omits the full order of battle, stacking, morale/cohesion, breakdowns, detailed logistical networks, stores, replacements, and the Italian pasta rule. Shared fallback artwork is distinguished by labels. Roads are decorative and do not change movement cost. The supply overlay shows distance, not guaranteed stocks or a traced route. Movement animation interpolates endpoints, not every hex of the engine's reachable path.

The 24 shared unit-role sprites in `public/sprites/` were generated with Amp's AI image-generation tool, then cropped and resized with transparency preserved. They are illustrative, not unique portraits, historically exact reconstructions, or extracted rulebook counters. The original four game sprites are unchanged; the catalog adds twenty role illustrations. The retained teaching map is original SVG artwork; the current review map uses the supplied source board under the private opt-in above. The original game and rulebook belong to their respective rights holders; this prototype does not establish a license for commercial distribution.

All 24 shared sprite roles have 18 upright directional views each in `public/sprites/directional/` (432 frames), and all are available as sandbox role units. The renderer completes the shortest-arc turn in place, then moves with a fixed heading. Turns use an even angular cadence at 2 ms per degree (80 ms minimum, 360 ms for a half-turn); travel eases independently over 420–1800 ms by distance. Already aligned units move immediately. Exactly one fully opaque directional view is shown at a time, with no crossfade, double exposure or flat image rotation. Reduced motion snaps immediately. Each atlas has six columns and three rows of 256px cells, ordered clockwise from east. Nominal 20° intervals and ground-contact alignment remain approximate illustrations, not calibrated 3D renders; discrete heading and detail changes remain visible. `public/data/directional-sprites.json` records role-art provenance and limitations, including near-duplicate views and ambiguous AA, engineer and warship bearings. Individual unit artwork uses the same atlas geometry but resolves through the identity inventory, not the shared-role path.

## Code

- `crates/cna-sim/src/sandbox/`: unchanged teaching-game state, geometry, legal moves, combat, supply, AI and outcomes. The crate root re-exports this API for the existing browser. `tests/sandbox_behavior.rs` preserves the sandbox regression assertions. `contract/` defines versioned authoritative-engine envelopes, not implemented official rules; see [command boundary v1](docs/contracts/sim-command-v1.md).
- `crates/cna-sim/build.rs`: conservative compile-time token lint rejects `sandbox` references anywhere under `src/official/`, including comments and unused source files. It is a direct-reference guard, not Rust name resolution; official modules must not use the crate-root compatibility re-exports either.
- `crates/frontend/src/board.rs` and `board/`: full-board review session, floating interface, keyboard dialogs and source-reference calendar. No sandbox state is imported.
- `crates/frontend/src/map/research.rs`: main tiled viewport, pointer/keyboard navigation and screen-space planning overlays.
- `crates/frontend/src/lib.rs`, `view.rs`, `map.rs`: entry point, shared icons and retained unmounted teaching UI/SVG renderer.
- `crates/frontend/src/motion.rs`: shortest-arc sprite turns and movement interpolation, with reduced-motion support.
- `crates/frontend/src/catalog.rs`: source catalog parsing, search, filters, and reference rendering.
- `crates/frontend/src/inventory.rs`: candidate identity search, unique-art readiness and per-unit asset resolution.
- `crates/cna-sim/src/sandbox/roles.rs`: explicitly fictional sandbox capabilities for ground, air, naval and support roles.
- `public/data/unit-catalog.json`: versioned source records, stat verification, and sprite metadata; embedded in the Wasm build, so rebuild after changing it.
- `src/style.css`: responsive field-map interface.
- `scripts/build-wasm.sh`: builds the Rust frontend and generates browser bindings in ignored `public/wasm/`.

`npm test` runs the private-review HTTP route tests and Rust workspace tests. `npm run test:private-review` checks image serving, path/method rejection, opt-in and production exclusion using temporary synthetic assets; no private artwork is required. `cargo clippy --workspace --all-targets -- -D warnings` checks Rust code. Keep the CLI version in setup and this document aligned with the exact wasm-bindgen dependency in `crates/frontend/Cargo.toml`.
