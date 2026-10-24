# CNA implementation plans

These four canonical plans were expanded by Claude Code and independently reviewed by Codex CLI on Kevin's Mac. They describe future work; they are not evidence that the game implements these rules.

| Workstream | Linear project | Orb | Milestones | Parent issues | Executable subtickets |
|---|---|---|---:|---:|---:|
| Map | [P-CNA-18](https://linear.app/littleblackhat/project/cna-map-af655b8ee559) | [Map orb](https://ampcode.com/threads/T-01a16d7a-76c8-7689-8e20-1bbc968a5f33) | 6 | 11 | 39 |
| Unit movement | [P-CNA-19](https://linear.app/littleblackhat/project/cna-unit-movement-8384a7c68199) | [Movement orb](https://ampcode.com/threads/T-01a16d7a-7f90-754e-8629-5581d9edab94) | 5 | 8 | 16 |
| Catalog and viewer | [P-CNA-20](https://linear.app/littleblackhat/project/cna-catalog-and-unit-viewer-5262b3b8dae6) | [Catalog orb](https://ampcode.com/threads/T-01a16d7a-87b7-7262-a044-ee9d1f30bff8) | 7 | 16 | 49 |
| Rules simulation | [P-CNA-21](https://linear.app/littleblackhat/project/cna-rules-simulation-91a776fdfa85) | [Simulation orb](https://ampcode.com/threads/T-01a16d7a-91e9-710b-918d-62d24e845c3f) | 9 | 83 | 272 |

Total: 27 native milestones, 27 corresponding epic tickets, 118 parent issues and 376 S/M subtickets: 521 tickets. Future art-wave expansion is intentionally gated on identity reconciliation and exact membership; 376 is not a promise that no further work will be discovered.

## Sources of truth

- `*.plan.json`: planner/refiner/describer outputs, local keys and prerequisites.
- `cna-dag.manifest.json`: cross-project prerequisites, global stages, file ownership, verification prerequisites and unresolved decisions.
- `*.map.json`: canonical-key-to-Linear-ID mappings. Preserve them when synchronizing; do not recreate the projects or tickets.
- `cna-reconciliation.md`: adopted and rejected review proposals, limitations and source decisions.
- `cna-linear-audit.json`: publication audit. All 521 tickets were re-read, including descriptions, hierarchy and milestones. All 1,717 explicit/inherited blocker edges were observed in remote relations, including the 80 manifest cross-project edges. High-fan-in relations were checked from both endpoints because the API truncates long relation lists.

Each Linear project also has a PRD and a **DAG and builder handoff** document with linked cross-project dependencies and ownership boundaries.

## Forward-only execution order

All 521 tickets and four projects now belong to the CNA team. UUIDs, descriptions, hierarchy, milestones, workflow/size labels and all 1,717 blocker edges were preserved. Use the refreshed `*.map.json` files for the new CNA identifiers.

`cna-execution-order.json` supplies the global execution rank and prerequisite wave for every ticket. Every blocker points from a lower rank/wave to a higher rank/wave. The audit also includes 494 child-to-parent completion edges: containers complete after their children; they are not executable work and do not block child starts.

The stricter audit found 24 valid dependencies running backward within the old simulation E/I/S numbering. Those keys are append-only source identities, not execution positions. The explicit topological order supersedes numeric key/Linear-ID sorting without deleting real prerequisites. The 521-node, 2,211-edge combined graph has no cycles, backward execution edges or backward milestone/stage dependencies. Evidence is in `cna-team-migration-audit.json`; cycle and reversed-rank negative controls were rejected.

Builders must choose ready S/M leaves from live Linear state, respecting cross-project blockers and actual availability of prerequisite code. The execution rank orders ready candidates; it does not require unrelated tickets to run serially or permit skipping unfinished blockers. Kevin authorized starting the build after this audit.

## Four builders, one owner per file

The simulation lane first publishes the minimal shared scaffold. Map and catalog publish independently owned contracts; simulation publishes route/projection fixtures; movement consumes those contracts. Later simulation-owned integration joins delivered components. No earlier producer milestone is made dependent on a later consumer milestone.

Only start ready S/M subtickets. Respect actual Linear blockers, including prerequisites inherited from parent tickets; these were explicitly copied onto descendants at publication. Complete grouping tickets only after every child is accepted. Do not execute a parent issue as an enormous implementation task.

The stock CLI adapter could not authenticate in either reviewed environment. Publication therefore used the connected Linear MCP transport, generated from canonical JSON with durable per-operation checkpoints. Do not blindly run a fresh adapter sync: preserve mappings, read existing issues first, and synchronize the manifest's foreign blockers as real relations. A plan-local ready queue alone is insufficient.

Shared entry points belong to simulation integration. A lane needing a boundary edit supplies a patch to that owner. Orbs do not share checkouts: transfer exact unpushed files or consume an explicitly published revision rather than assuming another orb's local `main` is `origin/main`.

## Visible progress and working portals

Kevin requires reviewable screenshots and a working portal, not only test-count updates. At each visual milestone, the owning orb must exercise the current rendered UI, inspect screenshots of representative affected states, and embed the final captures in its reply. Movement, rotation, hover, menus and other interactions also need an inspected real-pointer or keyboard recording. Save review media under `.amp/in/artifacts/`; send the exact paths to the parent so it can embed the evidence too.

Run previews as supervised services with `amp orb services ensure`, or `amp orb service start ... --portal` where no service is declared. Verify the service serves the current build, exercise its behavior in the browser, and share the exact returned public portal URL. Do not substitute localhost links, an untested stale portal, or screenshots from an older revision. State which features remain fixtures or teaching-sandbox behavior rather than official rules.

The simulation integration orb owns the combined review portal. Other lanes transfer their verified changes there and include their own working portal links when their UI diverges. An implementation milestone is not evidence that the parent thread's older preview has been updated. Source-art distribution and private-state protections still apply; do not bypass either to show progress. Nonvisual source/contract work needs no invented visual changes.

Kevin explicitly requested the supplied full map in the combined review portal on 2026-10-24. Serve the actual source-derived tiles there through an opt-in review configuration, rather than leaving the source-art placeholder. This request does not establish third-party redistribution rights or authorize public release: keep the default release guard and distribution policy unchanged, with review tiles outside `public` and `dist`. Verify the authenticated portal with real tiles and inspect overview, zoomed detail, narrow-layout screenshots and a pan/zoom recording. Displaying source artwork does not certify seam aliases, terrain data or official movement rules.

## Verification and certification

```sh
for plan in docs/planning/*.plan.json; do
  devkit pipeline validate --stage full --plan "$plan" || exit
done
python3 docs/planning/cna-dag-check.py
```

The validator includes container-completion and inherited-start edges. Negative controls rejected cycles, parent cycles, backward stages, ownership violations, missing verification producers and oversized leaves. Removing a redundant harness edge correctly remained valid when a transitive prerequisite still supplied it.

Future builder fixtures are not already-passing evidence. A fixture-only leaf may temporarily quarantine unimplemented rule tests, but implementation acceptance must execute the named tests, demonstrate the failing baseline where applicable, remove the relevant quarantine and reject zero-test or ignored-only success. Do not suppress failures to close a ticket.

Rules sections 1–65 are indexed; non-normative sections are explicitly not applicable. VASSAL is comparison evidence, not a rules oracle. Historical source conflicts, source-art distribution rights and unresolved reference data remain explicit gates. The first bounded target is Graziani's Offensive with detailed logistics and section 58 abstract air, not certification of the complete game.

## Review provenance

- [Claude Code planning on the Mac](https://ampcode.com/threads/T-01a16cef-608c-7531-a2da-fb7e8e387ca1).
- [Codex CLI independent review on the Mac](https://ampcode.com/threads/T-01a16cef-d891-7764-af86-e8a8010ef363): installed `codex-cli 0.163.0`, authenticated through ChatGPT, completed successfully. No claim is made about the server-selected model.
- [Parent planning/publication thread](https://ampcode.com/threads/T-01a16336-bc20-76f1-8657-2aa709091712).

Linear publication is complete. Gameplay implementation has not started through this planning operation. The parent checkout's plan files and Linear mappings remain local until separately delivered to Git; unrelated artwork changes were preserved.
