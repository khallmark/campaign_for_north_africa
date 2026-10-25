# Testing and contribution commands

## Prerequisites (Linux and macOS)

Use Node from `.node-version` (24.22.1), bundled npm, Git, Python 3.11+, and
Rust/rustfmt/Clippy plus `wasm32-unknown-unknown` from `rust-toolchain.toml`
(1.99.1). Install Rust with rustup and keep `$HOME/.cargo/bin` on PATH.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
rustup show                 # Installs the repository-pinned toolchain if needed
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.105 --locked
npm ci --include=dev
npx playwright install --with-deps chromium
npm run check:ci
```

Linux orbs can instead run `.agents/setup`. Run it again after adopting setup
changes; changing a feature branch does not update other orbs' snapshots.
The orb installer is Linux-specific; the commands above work on macOS too.
No API key, database, Claude/devkit authentication, or private tile directory is
needed. Do not start a review server to run tests. Keep TCP port 4175 free.

## Command contract

| Command | Scope and contract |
| --- | --- |
| `npm run check` | Fast default gate: format check, lint, then `npm test`. No browsers or source regeneration. Initial Rust compilation can still take minutes. |
| `npm test` | Credential and distribution guards, `cargo test --workspace --locked` (including doctests), then Node functional tests. Source-dependent tests are explicitly ignored. |
| `npm run test:unit` | Rust library and binary unit tests; focused developer command, not a substitute for the default guards. |
| `npm run test:integration` | All Cargo integration targets under `crates/*/tests`; synthetic map/tile, catalog, contract and sandbox behavior. |
| `npm run test:functional` | Node's built-in runner discovers `tests/*.test.mjs`: synthetic guard tests, plus PR3's HTTP route tests when merged. No new framework. |
| `npm run test:e2e` | Guards, then the existing Playwright suite. Its webServer builds guarded production Wasm/Vite assets and starts/stops a fresh preview. |
| `npm run test:browser -- ...` | Existing low-level Playwright entry point, retained for compatibility. Prefer `test:e2e` so credential guards also run. |
| `npm run check:ci` | Default gate plus E2E; CI runs this on both Linux and macOS. |
| `npm run format:check` / `npm run format` | Check / apply rustfmt across the workspace. Formatting is Rust-only; retain existing JS/TS/Python style. |
| `npm run lint` | Strict native all-target Clippy, strict Wasm frontend Clippy, and `lint:scripts`. |
| `npm run lint:scripts` | Syntax-only checks for authored JS/MJS, Bash and Python without importing/executing scripts. Not ESLint, Python semantic lint, or TypeScript type-checking. Playwright compiles and executes the TS specs. |
| `npm run check:guards` | Scan tracked credential/save markers without logging contents; reject uncleared tiles, private source directories, symlinks and byte-identical locked sources in `public/` and `dist/`. |
| `npm run check:sources` | Explicit full source inventory, size and SHA256 validation. Missing files, symlinks, drift and LFS pointers fail. No auto-regeneration. |
| `npm run test:sources` | Full integrity preflight, all ignored Rust tests in release mode (including 66-megapixel tile regeneration), then `test:source-data`. Deliberately expensive and private-source-dependent. |
| `npm run test:source-data` | Python unittest discovery of `scripts/test_unit_data.py` using `.venv`. Runs the integrated PR2 importer/schema suite, including its read-only regeneration check. Requires restored/hydrated sources. |

Focused examples:

```sh
cargo test --locked -p cna-sim --test sandbox_behavior
npm run test:e2e -- tests/browser/map-viewport.spec.ts --project=chromium-1280x800-dpr1
```

Playwright keeps the existing four viewport/DPR projects (2458×1358 and
1280×800, DPR 1 and 2), two workers, zero retries and `forbidOnly`.
Tests intercept map requests with synthetic art. Failure videos, traces and
screenshots stay in ignored `test-results/`; CI does not upload them or `dist/`.
No visual UI baseline or arbitrary coverage percentage is imposed.

## Verify without original artwork

CI checks out no original image directory or PDFs, does not hydrate LFS, and
asserts their absence before `check:ci`. Reproduce in a **new disposable clone**:

```sh
GIT_LFS_SKIP_SMUDGE=1 git clone --no-checkout --branch YOUR_BRANCH REPOSITORY_URL cna-check
cd cna-check
git sparse-checkout set --no-cone '/*' '!/docs/CNAv2.1.0/images/' '!*.pdf'
GIT_LFS_SKIP_SMUDGE=1 git checkout
npm ci --include=dev
npm run check:ci
```

Tracked source text and source-lock metadata remain available. Sparse files are
still credential-scanned from Git's indexed blobs, without materializing images;
sparse exclusion is not a scanner bypass. Ordinary missing tracked files fail.
The Git repository still contains source objects: this is an art-free working
tree, not a sanitized redistributable clone.

The distribution guard needs only source-lock fingerprints, not private artwork.
It detects exact copies, not transformed/re-encoded derivatives; review provenance
before adding assets. A map-tile grant does not authorize copying original source
files. The explicit `--local-only` override remains private research, is never
used by default checks/build/CI, and does not change the grant policy.

## Explicit source work

Use only an authorized source checkout. These commands do not publish artifacts:

```sh
git sparse-checkout disable       # Only if this checkout was sparse
git lfs install --local
git lfs pull
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-test.txt
npm run test:sources
```

Install Git LFS first if absent. Python test dependencies are pinned, including
transitive packages. The orb equivalent is `CNA_SETUP_SOURCES=1 .agents/setup`
after restoring sparse paths. Missing source inputs fail the integrity preflight;
do not suppress failures, refresh locks to hide drift, or run source extraction
in default CI. Regenerated tiles belong outside `public/` and `dist/`.

## Worker adoption and pending PR reconciliation

Workers should adopt this setup after the integration owner merges it, rerun
setup or `npm ci`, run `npm run check`, then `npm run test:e2e` before review.
Add behavior tests in the existing owner crate's unit/integration files, Node
`tests/*.test.mjs`, or Playwright `tests/browser/*.spec.ts`. Test observable
outcomes with inputs that distinguish likely regressions. No quota is required.
Source-dependent Rust tests must have an explanatory `#[ignore]` and run under
`test:sources`; fast synthetic counterparts should remain in the default lane.

This branch is based on the integration branch with PR2 merged; it does not
duplicate PR2's commits or include PR3. On combining:

- **PR3:** preserve `test:private-review: node --test tests/private-review.test.mjs`
  as a focused alias. Keep this branch's `test`, `pretest` and `test:functional`:
  the functional glob already runs PR3's route suite, so do not run it twice.
  Preserve PR3's canonicalized `realpath(await mkdtemp(...))` fixture root.
  Playwright automatically discovers its browser additions.
- **PR2 (integrated):** `test:source-data` runs its existing importer/schema tests
  directly. That suite rebuilds from original images/PDFs, so it stays out of
  default CI. Its owner can add independent synthetic importer tests to the fast
  lane later; do not label the current full-source suite art-free.
- Expect a `package.json` conflict with PR3's `test`/`pretest` edits and possibly
  README context overlap. Keep both feature documentation and this command
  contract. Do not import either worker's implementation into this setup PR.

CI uses read-only repository permissions and does not deploy or publish. Merges,
private source grants and production distribution remain the integration owner's
decisions. Changes to guards must retain their negative/regression tests.
