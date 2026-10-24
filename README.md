# The Campaign for North Africa — Field Command

A small browser-playable prototype inspired by SPI's _The Campaign for North Africa_. Command four Commonwealth formations on a schematic Tobruk-frontier hex map against three automated Axis formations. Capture Tobruk within 12 operations stages.

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
Each lane owns its feature specs; the shared smoke test only checks startup.

## Amp orbs

Amp runs `.agents/setup` when preparing a fresh or stale project snapshot. It installs the pinned Node.js release only when needed, checks the official download checksum, runs `npm ci`, and prepares Rust, the Wasm target, and the matching wasm-bindgen CLI. Matching snapshots skip setup and retain the installed dependencies. The build script makes Rust's user-local tools available to supervised services.

```sh
.agents/setup             # Safe to rerun manually; installs tools and dependencies
amp orb services ensure   # Starts the supervised Vite server and prints its portal URL
```

The development service is declared in `.amp/services.yaml`; setup does not start it. No resume hook is needed because there are no credentials, tunnels, or databases to refresh. Generated `.amp/portals/` state stays untracked. Setup changes must reach the project's default branch before future orbs can use them.

### Authorized private full-map review

The full-board research viewer can serve source-derived tiles in an explicitly
authorized review orb without putting them in `public/` or `dist/`:

```sh
cargo run --locked --release -p cna-map --bin cna-map-tiles -- --output /tmp/cna-private-review-map
# Set CNA_PRIVATE_REVIEW_TILES=/tmp/cna-private-review-map in ignored .env.local.
npm run build
amp orb service restart cna
amp orb services ensure
```

Expand **Full-board map · research** below the teaching sandbox. The opt-in Vite
development middleware serves only the manifest and numbered PNG tiles at
`/tiles/map/`, with private/no-store responses. It is absent from production
builds and previews. Remove the local environment setting and restart to disable
it; regenerate the pyramid if the temporary directory is lost. Do not commit
the opt-in or copy the tiles into release assets.

Kevin authorized this source artwork for his existing review portal on
2026-10-24. That is not a third-party redistribution grant or public-release
authorization: `docs/map/distribution.json` remains pending, and the default
build still rejects uncleared tiles in public assets. Historical seams, terrain,
scenario identities and official rules remain unverified.

## Play

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

The 24 shared unit-role sprites in `public/sprites/` were generated with Amp's AI image-generation tool, then cropped and resized with transparency preserved. They are illustrative, not unique portraits, historically exact reconstructions, or extracted rulebook counters. The original four game sprites are unchanged; the catalog adds twenty role illustrations. The map is original SVG artwork. The original game and rulebook belong to their respective rights holders; this prototype does not establish a license for commercial distribution.

All 24 shared sprite roles have 18 upright directional views each in `public/sprites/directional/` (432 frames), and all are available as sandbox role units. The renderer completes the shortest-arc turn in place, then moves with a fixed heading. Turns use an even angular cadence at 2 ms per degree (80 ms minimum, 360 ms for a half-turn); travel eases independently over 420–1800 ms by distance. Already aligned units move immediately. Exactly one fully opaque directional view is shown at a time, with no crossfade, double exposure or flat image rotation. Reduced motion snaps immediately. Each atlas has six columns and three rows of 256px cells, ordered clockwise from east. Nominal 20° intervals and ground-contact alignment remain approximate illustrations, not calibrated 3D renders; discrete heading and detail changes remain visible. `public/data/directional-sprites.json` records role-art provenance and limitations, including near-duplicate views and ambiguous AA, engineer and warship bearings. Individual unit artwork uses the same atlas geometry but resolves through the identity inventory, not the shared-role path.

## Code

- `crates/cna-sim/src/sandbox/`: unchanged teaching-game state, geometry, legal moves, combat, supply, AI and outcomes. The crate root re-exports this API for the existing browser. `tests/sandbox_behavior.rs` preserves the sandbox regression assertions. `contract/` defines versioned authoritative-engine envelopes, not implemented official rules; see [command boundary v1](docs/contracts/sim-command-v1.md).
- `crates/cna-sim/build.rs`: conservative compile-time token lint rejects `sandbox` references anywhere under `src/official/`, including comments and unused source files. It is a direct-reference guard, not Rust name resolution; official modules must not use the crate-root compatibility re-exports either.
- `crates/frontend/src/lib.rs`: Rust browser input, dialogs, and session state.
- `crates/frontend/src/view.rs`: interface markup, formation panel, and dialogs.
- `crates/frontend/src/map.rs`: persistent keyed SVG map and sprite rendering.
- `crates/frontend/src/motion.rs`: shortest-arc sprite turns and movement interpolation, with reduced-motion support.
- `crates/frontend/src/catalog.rs`: source catalog parsing, search, filters, and reference rendering.
- `crates/frontend/src/inventory.rs`: candidate identity search, unique-art readiness and per-unit asset resolution.
- `crates/cna-sim/src/sandbox/roles.rs`: explicitly fictional sandbox capabilities for ground, air, naval and support roles.
- `public/data/unit-catalog.json`: versioned source records, stat verification, and sprite metadata; embedded in the Wasm build, so rebuild after changing it.
- `src/style.css`: responsive field-map interface.
- `scripts/build-wasm.sh`: builds the Rust frontend and generates browser bindings in ignored `public/wasm/`.

`npm test` runs the Rust workspace tests. `cargo clippy --workspace --all-targets -- -D warnings` checks Rust code. Keep the CLI version in setup and this document aligned with the exact wasm-bindgen dependency in `crates/frontend/Cargo.toml`.
