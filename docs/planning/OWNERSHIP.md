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

Per-owner frontend modules and CSS are created by the behavior-preserving
E1.I1.S3 split; their listing above does not mean they already exist. `motion.rs`
remains movement-owned and is not part of that split.

Shared entry-point or dependency changes go through simulation. Other lanes
supply a scoped patch rather than editing those files independently. Transfer
exact verified patches/bundles with their base revision: another orb's local
branch is not `origin/main`. Do not overwrite newer work when applying a transfer.
