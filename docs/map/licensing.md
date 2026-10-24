# Map artwork distribution

**Status: pending Kevin decision. No redistribution grant is recorded.**
The machine-readable source of this status is `docs/map/distribution.json`.
Do not change it to `granted` merely because the source image is in Git, a
VASSAL module is downloadable, or a local build succeeds.

## Rights notices and contributors

`docs/CNAv2.1.0/moduledata` identifies the work as
`(c) 1979 SPI/(c) 2022 White Box Games`. These are the named rights holders in
the supplied material, not independently verified current ownership.

`Read me.txt`, `playnotes.txt` and `changelog.txt` credit:

- **Pete Phillipps, March 2020:** original VASSAL module.
- **Mitch Guthrie, September 2021:** revised module, map correcting scan
  misalignments/color variances, and SVG counters.
- **Norman Harman, September 2024:** v2.1 updates, including starting dumps,
  city fortifications and port-efficiency labels on the map.

None of these supplied notices states a redistribution license for this web
application. Credits do not substitute for permission. The already-corrected
D2228/2328 escarpment is source history, not a license or a new map edit.

## Local research versus public delivery

The project permits private technical inspection of its supplied files and
local, ignored generated tiles for this research workflow. That operational
choice is **not a legal determination or a grant from a rights holder**.
Keep tiles in ignored `public/tiles/`; do not commit or publish them, share a
public tile-hosting portal, or deploy a research bundle while status is pending.
The committed source image is unchanged by this work.

`npm run build` checks `public/tiles/` before bundling and `dist/tiles/` afterward.
With `pending` or `local_only` status, a nonempty tile tree causes failure;
empty directories are allowed. Links count as payload, including broken links.
The guard does not delete files and does not prove licensing for assets moved
elsewhere, counters, other artwork, or manual deployments bypassing this command.

```sh
# Check the ordinary production bundle (default: dist).
cargo run --locked -p cna-map --bin map-census -- --release-check

# Inspect source assets before bundling.
cargo run --locked -p cna-map --bin map-census -- --release-check --bundle-dir public

# Explicit private-research override; not a release approval.
cargo run --locked -p cna-map --bin map-census -- --release-check --bundle-dir public --local-only
```

`--local-only` never changes the recorded status, is accepted only with
`--release-check`, and is deliberately absent from the production build script.
Private research can use the development server; the check alone does not
generate tiles or build a bundle. The tile generator remains a separate task.

## Decision Kevin must record

Either keep `pending`, choose `local_only` (public distribution still blocked),
or record a verified grant covering the source map and derivative tiles for the
intended audience/use. A grant decision must cite the permission document,
rights holder(s), scope, required credits, limitations and decision date here,
then set `status` to `granted` and `grant_reference` to that record's location.
The checker requires a nonblank reference for `granted`; an unknown status,
malformed policy or missing policy fails closed. It cannot authenticate a grant.

This task records the gate; it does not negotiate permission or authorize
publication. Catalog/counter-art distribution remains owned by the catalog lane.
