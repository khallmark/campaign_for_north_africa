# CNA workstream ownership

The exact path boundaries and one-time scaffold exceptions are recorded in
`cna-dag.manifest.json`. Execution uses `cna-execution-order.json` rank/wave and
live Linear blockers, never numeric ticket order. A completed prerequisite also
requires its exact code in the consumer's checkout.

| Owner | Files and responsibilities |
| --- | --- |
| Simulation | `crates/cna-sim/`, workspace and dependency manifests/lockfiles, `package.json`, browser harness configuration, `.agents/setup`, `README.md`, `index.html`, frontend `lib.rs` and `view.rs` compositor, `src/style.css` shell, rules/source adjudication and final integration |
| Map | `crates/cna-map/`, frontend `map.rs`, map CSS, geometry, map data, tiles and map browser tests |
| Catalog | `crates/cna-catalog/`, frontend `catalog.rs`, `inventory.rs`, `catalog_view.rs`, catalog CSS, identity/catalog data, sprites and catalog browser tests |
| Movement | Frontend `motion.rs`, `movement/`, `inspect.rs`, `view/panel.rs`, movement CSS and movement browser tests |

Simulation creates the empty map/catalog crates once in E1.I1.S1 (CNA-346),
including their version constants and the failing tile-generator stub. Ownership
then passes to the named lanes; this scaffold does not define their contracts.
The frontend depends on both crates but does not consume new behavior yet.

The behavior-preserving E1.I1.S3 split creates the owner modules above.
`view/panel.rs` renders formation details, `movement/mod.rs` renders order
controls, and `inspect.rs` resolves the selected formation and inspection labels.
`catalog_view.rs` supplies catalog/inventory content to the shared dialog shell.
`motion.rs` remains movement-owned and is not part of that split.

`src/styles/map.css`, `src/styles/movement.css`, and `src/styles/catalog.css`
contain owner component styles. The shell imports them and retains shared
typography, controls, page/modal framing, and coordinated responsive layout
overrides. Changes to those shared breakpoints still go through simulation;
new component-local styles belong in the owner stylesheet.

Shared entry-point or dependency changes go through simulation. Other lanes
supply a scoped patch rather than editing those files independently. Transfer
exact verified patches/bundles with their base revision: another orb's local
branch is not `origin/main`. Do not overwrite newer work when applying a transfer.
