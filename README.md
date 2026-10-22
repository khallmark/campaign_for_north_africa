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

## Amp orbs

Amp runs `.agents/setup` when preparing a fresh or stale project snapshot. It installs the pinned Node.js release only when needed, checks the official download checksum, runs `npm ci`, and prepares Rust, the Wasm target, and the matching wasm-bindgen CLI. Matching snapshots skip setup and retain the installed dependencies. The build script makes Rust's user-local tools available to supervised services.

```sh
.agents/setup             # Safe to rerun manually; installs tools and dependencies
amp orb services ensure   # Starts the supervised Vite server and prints its portal URL
```

The development service is declared in `.amp/services.yaml`; setup does not start it. No resume hook is needed because there are no credentials, tunnels, or databases to refresh. Generated `.amp/portals/` state stays untracked. Setup changes must reach the project's default branch before future orbs can use them.

## Play

- Select a Commonwealth formation on the map or in the order of battle.
- **Move:** choose a highlighted destination. Desert costs 1 CP per hex, ridges 2; vehicles use 8 fuel per CP. Occupied hexes and water are impassable. Entering an enemy-adjacent hex ends that move order.
- **Assault:** choose an adjacent enemy and confirm the damage preview. Costs 2 CP and 20 ammunition. A surviving defender returns fire. Combat is deterministic.
- **Dig in:** spend 1 CP for 10 damage protection until moving or attacking.
- **Resupply:** spend 1 CP within 2 hexes of a stocked truck or the frontier depot. Trucks transfer up to 25 of each resource, limited by recipient capacity; the depot refills all stores. Trucks need at least 25 of each resource to act as a supply source.
- **End stage:** Axis formations move and attack; supply upkeep consumes water and vehicle fuel, then CP refreshes. Every third incoming stage is hot. Units at zero water cannot move/assault and lose strength at upkeep.
- Move a surviving combat formation into Tobruk to win. Lose after stage 12 or when all Commonwealth combat formations are destroyed.

Map controls toggle the grid and geometric supply range. Zoom centers on the selected formation. All buttons, formations, and highlighted destinations support keyboard activation. Escape cancels the current order or closes a dialog.

## Scope and source material

The supplied rulebook text is in `docs/rules.txt`. This prototype adapts the ideas of operations stages (§5), capability points (§6), hex movement and enemy zones (§8), close assault (§15), fuel loss (§49), and water supply (§52). Three stages represent one week; hexes are nominally 8 km.

**This is not a rules-complete implementation or a historical scenario.** Forces, positions, terrain, timings within stages, values, weather cycle, and combat formulas are invented for a short playable demonstration. It omits the full order of battle, stacking, morale/cohesion, breakdowns, air and naval warfare, detailed logistical networks, stores, replacements, and the Italian pasta rule. The infantry sprite is shared between factions; labels distinguish sides. Roads are decorative and do not change movement cost. The supply overlay shows distance, not guaranteed truck stocks or a traced route.

Unit sprites in `public/sprites/` were newly generated with Amp's AI image-generation tool from a four-part transparent sprite atlas, then cropped and resized. They are illustrative, not historically exact or extracted from the rulebook. The map is original SVG artwork. The original game and rulebook belong to their respective rights holders; this prototype does not establish a license for commercial distribution.

## Code

- `crates/cna-sim/`: dependency-free game state, hex geometry, legal moves, combat, supply, AI, outcomes, and public-action regression tests.
- `crates/frontend/src/lib.rs`: Rust browser input, dialogs, and session state.
- `crates/frontend/src/view.rs`: interface markup, formation panel, and dialogs.
- `crates/frontend/src/map.rs`: persistent keyed SVG map and sprite rendering.
- `crates/frontend/src/motion.rs`: shortest-arc sprite turns and movement interpolation, with reduced-motion support.
- `src/style.css`: responsive field-map interface.
- `scripts/build-wasm.sh`: builds the Rust frontend and generates browser bindings in ignored `public/wasm/`.

`npm test` runs the Rust workspace tests. `cargo clippy --workspace --all-targets -- -D warnings` checks Rust code. Keep the CLI version in setup and this document aligned with the exact wasm-bindgen dependency in `crates/frontend/Cargo.toml`.
