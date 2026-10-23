# Distinct identities and unique artwork: work in progress

`public/data/unit-identities.json` separates source rows, unit identities, equipment models and artwork. It is **not a certified complete roster**. The earlier 24 shared role illustrations do not satisfy the requirement for unique artwork per distinct unit.

## Inventory boundary

The 1,450 catalog rows contain 52 role descriptions, not individual units. The 124 characteristics profiles are also not identities. Of the remaining 1,398 rows, 102 aircraft load/configuration rows become 78 nationality-scoped aircraft model candidates. Eighteen source-backed alias/redesignation merges leave **1,356 candidates**:

| Kind | Candidates | Meaning |
|---|---:|---|
| Formation | 1,034 | OA identities and counter-only formations, including provisional identifications |
| Squadron ground support | 132 | Named/numbered SGSUs, not aircraft models |
| Naval | 20 | Named vessels |
| Support | 11 | Named/lettered support counters, some game-specific rather than historical formations |
| Equipment | 81 | Printed equipment rows; some aggregate classes, not distinct vehicle models |
| Aircraft | 78 | Model/operator identities; load variants stay in `configurations` |

Thus 1,197 candidates concern organizations or counters and 159 concern equipment/model rows. Neither 1,450 nor 1,356 is a verified count of unique historical units. Equipment spelling/variant aliases, battlegroups, administrative pieces and the full 1,800-counter reconciliation remain unfinished. The count can change with source-backed reconciliation.

## Resolved and unresolved identities

Physical PDF page numbers are one-based and refer to the supplied scans, not printed folios. Exact catalog citations remain available through `source_refs` and `source_entry_ids`.

- **13th DCO Lancers:** pp.127-128 explicitly describe the same unit arriving attached, not assigned, to 18th Indian Brigade. Both rows map to one identity.
- **I(L), II(L), IX(L), II(M) tank battalions:** pp.147-150 and157 explicitly identify attached and parent-formation listings. These are four units, not eight.
- **Eight CCNN identities:** division tables on pp.150-152 and the unassigned table on p.161 identify the same attached 7th/9th Legion HQs and 7th/59th/60th/32nd/33rd/56th battalions.
- **San Giorgio:** p.155 note c identifies the Tobruk entry as the partially sunk cruiser. It maps to the named naval identity, not an additional land battalion.
- **5th Light / 21st Panzer HQ:** p.162 notes a-b explicitly replace the HQ counter at reorganization. One canonical lineage retains two `counter_states`; source rows and differing organization remain distinct. This does not collapse subordinate units or make both states simultaneously deployable.
- **I(M) Tank Battalion / Bn:** pp.149 and160 explicitly link the Libyan Tank Command unit and its 1st CCNN attachment. The expanded word is not a new identity.
- **22nd / 200th Guards Brigade:** p.121 explicitly says two counters are provided for the same unit and describes its redesignations. Counter states remain separate within one identity.
- **I/75 / II/155 Artillery Battalion:** p.165 note e explicitly replaces the former counter with the latter upon conditional reassignment. One identity retains the two source states rather than inventing simultaneous deployments.
- **200th Pioneer:** both pp.162 and164 visibly print 200, not 220, under different divisions and arrival values. Without a linking note these remain separate provisional candidates. Do not silently correct the printed source from outside assumptions.

`identity_redirects` preserves all 18 retired initial inventory IDs. Source rows retain their original IDs. A merge does **not** merge numeric stats or imply a single scenario state.

## Renderer and simulation contract

The implementation owner reads the JSON directly with Rust/serde. `docs/unit-identities.schema.json` describes the shape. Existing field types are unchanged; extra fields are optional for consumers.

- `id` is a stable identity key. Resolve retired IDs through `identity_redirects` before lookup.
- `role_id` is one of the existing 24 illustrative role IDs, or null when unsupported. It selects a fallback/capability family, never proves unique artwork or historical composition.
- `entity_kind`, `identity_status`, `source_entry_ids`, and `source_refs` distinguish evidence from presentation.
- `sprite.status` is `missing`, `candidate`, or `ready`. Missing paths are null. Only `ready` may replace the shared fallback automatically. Keep a fallback if loading fails.
- Ready paths are `/sprites/units/{id}/portrait.png` and `/sprites/units/{id}/atlas.png`. They are identity-specific files, not copies or recolors of a shared role atlas.
- Atlases use 18 genuine upright views in a 6×3 grid of 256px cells, total 1536×768 RGBA. Headings are nominally clockwise from east in 20° increments. Crop/select frames; never rotate the whole image or subtract a 150° baseline. The old 150° metadata describes old single-view art only.
- The seven original scenario IDs remain `prototype_proxy` with null `identity_id`. Aggregate brigades/divisions, RASC and Tobruk Garrison must not be automatically mapped onto individual HQ or support counters.

## First unique-art batch

**Forty-nine identities have reviewed illustrative artwork; 1,307 candidates still have none.** These incremental batches do not fulfill all-unit artwork coverage. The first batch contains:

1. `unit-commonwealth-2nd-bays`: independently generated red-pennant cruiser tank, pale rear tarpaulin, sand/sage pattern.
2. `unit-commonwealth-10th-royal-hussars`: independently generated blue-pennant cruiser tank, rear crate, spare tracks on the nose, ochre pattern.

Painter generated each 18-view sheet, then a separate six-view rear strip corrected duplicate/reversing rear angles. The manifest retains generation attachment URLs. Packing extracts alpha components, preserves edge alpha with a two-pixel expansion, scales all views consistently and aligns silhouette bottoms to y240 with center x128. Portraits are actual frame7 crops. No flat rotations, reflections or recolored substitutes were used.

The scan on p.116 supports the two unit identities and weapon-system TOE assignments: 2nd Bays has `8 × Crusader Mk I, 2 × A13`; 10th Royal Hussars has `9 × Crusader Mk I, 1 × A13`. These are printed TOE strength points, not literal vehicle counts. The generated tanks are representative illustrations, not exact model reconstructions. Pennants, camouflage, stowage and vehicle markings are artistic differentiation, **not sourced historical insignia**.

`ready` means the files meet the crop/alpha contract and passed upright, broad-heading visual review. It does not by itself mean calibrated 3D or gameplay-verified. Some adjacent angles remain nearly repeated; apparent angular steps and details vary. These atlases alone do not solve animation smoothness.

Both initial identities now have `gameplay_verified: true` with an explicit limited scope and evidence reference. The implementation owner deployed and moved each through searched identity controls in Chromium. This catalog thread independently inspected the transferred `.amp/in/artifacts/unique-unit-movement.mp4`: both sprites stay upright, labels remain horizontal and no atlas-cell bleed is visible. The owner additionally reports distinct identity atlas paths, no unit rotate transforms, zero blend at rest and no page errors after fixing a UI input-blur panic. Those DOM/console claims are owner-reported, not inferred from the recording. This is not exhaustive heading coverage or a smoothness certification.

## Second unique-art batch

Four more stable canonical identities now have a portrait and 18-view atlas:

- `unit-commonwealth-9th-lancers`: angular cruiser tank, ochre pennant, brown bands, green bedroll and rear spare wheels. The first generated sheet had inconsistent hull/barrel direction and was rejected; the replacement used the reviewed 2nd Bays poses as a guide, not copied pixels.
- `unit-commonwealth-12th-lancers-bn`: four-wheel armored car, rear spare wheel, folded side canvas and whip aerial. Genuine rear views 14/15 and 16/17 were reordered after inspection.
- `unit-commonwealth-1st-rifle-brigade`: bush-hatted rifleman with olive shoulder blanket and tan pack. Frames 6-11 were regenerated to restore front-left through back-left chest/backpack visibility.
- `unit-commonwealth-1st-armored-div-hq`: map-reading officer with radio pack and leather mapcase. The manifest records exact frame selection from the original sheet and corrective strip.

All four passed light/dark visual checks for upright views, broad clockwise ordering, complete silhouettes and lack of cell bleed. Adjacent poses remain uneven and near-repeated. All four now have scoped `gameplay_verified: true`: parent reports searched deployment, distinct identity paths, movement, retained nodes, no unit rotate transforms and zero blend at rest. Parent also reports 25 tests/build/schema/108-cell checks passing and no browser errors. This thread independently inspected `batch02-movement.mp4`, which visibly exercises only 9th Lancers; the other three movement/DOM checks are parent-reported. This does not certify every heading or smoothness. Artwork readiness does not promote provisional historical identity metadata; no source stats or scenario values changed.

These are distinct illustrations, not historically sourced uniform, insignia or vehicle details. The 76th Anti-tank Regiment request was rejected by Painter; that identity remains missing with a recorded blocker. No shared illustration was substituted as unique coverage. Other identities remain available for continued production.

## Third unique-art batch

Four further identities retain the same portrait/frame geometry and canonical IDs:

- `unit-commonwealth-2nd-armored-bde-hq`: unarmed radio liaison jeep with two aerials, rear canvas radio cover and front canvas roll.
- `unit-commonwealth-1st-support-group-hq`: service-capped operator with handset and hip satchel. The last four views were regenerated to complete the rear-to-right-profile turn.
- `unit-commonwealth-2nd-krrc`: helmeted rifleman with square haversack and waist bedroll. Actual views 10/11 were swapped to remove a small heading reversal.
- `unit-commonwealth-4th-hussars`: cream/olive tank with turret spare wheel and rear toolboxes. The last four views were regenerated to improve the wrap to frame0.

All four passed inspected light/dark contact sheets after correction. Parent now reports searched deployment with distinct identity paths and movement through 59–60 sampled positions with 19–20 fractional blends, retained nodes, no rotation and zero rest blend. Each movement spent 1 CP; 4th Hussars spent 8 fuel. Parent reports no browser errors and 25 tests/build/schema/180-cell checks passing, inspected four gameplay screenshots and recorded/inspected 4th Hussars turning in `batch03-movement.mp4`. These checks are parent-reported, not independently repeated here. Scoped `gameplay_verified` is now true; it does not certify all headings, smoothness or historical accuracy. These are illustrative representations, not historical equipment assignments, uniform details or exact 20-degree turn models. No flat rotations, mirroring or copied shared-role files were substituted.

## Fourth unique-art batch

Four Axis identities now have separate portrait/atlas files; existing canonical IDs and all source mappings remain unchanged:

- `unit-italian-vii-m-tank-bn`: sand riveted tank with rear crate/fuel cans and front spare tracks. Fresh frames 14–17 improve rear-to-east closure.
- `unit-italian-viii-m-tank-bn`: gray/sand tank with rear canvas roll and turret spare wheel. Original frames 11/12 were swapped to remove a rear-heading reversal.
- `unit-german-i-5-panzer-bn`: sandy tank with rear metal basket/crates, turret tarp and front spare tracks. Original rear views disagreed with hull/gun direction and ended facing left; six fresh views replace frames 12–17.
- `unit-german-ii-5-panzer-bn`: ochre/brown striped tank with rear spare wheels and side canvas roll.

The Italian identities cite physical scan p.147 and the German identities p.162. These citations support catalog identities, not the invented paint, stowage or exact vehicle models. Identity status remains provisional; artwork does not promote source confidence. No numeric source stats or scenario values changed.

All four packed sheets were inspected on light and dark backgrounds. They remain upright, broadly progress east/front/west/rear/east and show no gross reversals, clipping, conspicuous halos or neighboring-cell bleed. **Several poses nearly repeat and angular steps are uneven**, including near-equal east views around the loop boundary. The manifest records per-atlas limitations; these are not calibrated 20-degree turn models. The existing Painter rejection for 76th Anti-tank Regiment remains unresolved and excluded from ready coverage.

Parent now reports all four searched deployments under Player controlled resolving distinct portrait/atlas paths and moving 11,7 → 12,7. DOM samples show 59–61 distinct positions, 20–21 fractional blends, retained selected nodes, no rotation and zero rest blend; each spent 1 CP and 8 fuel, with no browser errors. Parent inspected four screenshots and II/5 turning footage in `batch04-movement.mp4` plus a 5fps contact sheet. Only II/5 has footage; the other three checks use DOM/screenshots. Parent reports 25 tests, build, schema and 252-cell geometry/alpha checks passing. These are bounded parent-reported checks, not independently repeated here, exhaustive headings or historical certification. Scoped `gameplay_verified` is true.

## Fifth unique-art batch

Four reconnaissance identities have separate artwork:

- `unit-italian-reco-nizza-cavalleria`: four-wheel sand/olive car with rear spare and toolbox, identity citation p.147.
- `unit-italian-reco-lanceria-di-novara`: six-wheel cream/brown car with green rear canvas, identity citation p.148. Frames 15–17 use correction-strip views 1–3; original frame 14 remains, removing a slight closing reversal.
- `unit-german-3rd-aufklarungs-bn`: eight-wheel sand/green car with rectangular frame aerial, identity citation p.162.
- `unit-german-33rd-aufklarungs-bn`: open-turret four-wheel car with blue-gray canvas and tan cases, identity citation p.163. Frames 14–17 were regenerated to close toward right profile.

Vehicle designs, paint and generated markings are illustrative choices, **not sourced equipment assignments**. Each has its own generated pixels and canonical paths; no shared-role recolors, flat rotations or mirrors count toward coverage. Light/dark inspection found upright broad turning order, transparent aerial openings and no clipping, conspicuous halos or neighboring-cell bleed. Near-repeat spans and uneven yaw remain, including imperfect hull/barrel progression near the rear view for 33rd Aufklarungs; per-entry limitations record these. Source catalog, identity counts, aliases and scenario stats remain unchanged.

Parent reports searched Player-controlled deployment and movement for all four: distinct portrait/atlas paths, 11,7 → 12,7, 59–60 sampled positions, 20 fractional blends, retained nodes, zero rotation/rest blend and 1 CP/8 fuel cost. No browser errors; 25 tests/build/schema/324-cell checks pass. Parent inspected four gameplay screenshots and recorded/inspected only 33rd in `batch05-movement.mp4` and a 5fps sheet; other three checks use DOM/screenshots. Scoped `gameplay_verified` is true, recording these as parent-reported, not independently viewed here or exhaustive art certification.

## Sixth unique-art batch

Four infantry identities receive distinct illustrative figures:

- `unit-italian-i-35-infantry-bn`: sun helmet, sand clothes, dark haversack and canvas rolls, identity citation p.149.
- `unit-italian-ii-35-infantry-bn`: sidecap, olive shirt, pale backpack and lower dark bedroll, identity citation p.149. Initial sheet lacked front-left views and was rejected; a regenerated sheet restores them, with original frames 12–14 reordered as 13,14,12.
- `unit-german-i-104-schutzen-bn`: helmet, dark pack, gray blanket and vertical shovel, identity citation p.163. Frames 12/13 were swapped to remove a small rear reversal.
- `unit-german-ii-104-schutzen-bn`: fieldcap, green-gray shirt, brown pack and metal hip tin, identity citation p.163. Four new final frames improve right-profile closure.

These are not sourced historical uniform or weapon assignments. Light/dark inspection confirms upright broad directional coverage, complete silhouettes and no obvious halos, clipping or bleed. Near-repeat spans remain substantial. None is a calibrated 18-angle model or smoothness certification. No source identities, alias mappings, numeric stats or application code changed.

Parent reports all four searched Player-controlled deployments resolving distinct paths and moving 11,7 → 12,7 through 59–60 positions and 20–22 fractional blends, retained nodes, zero rotation/rest blend, 1 CP cost, no applicable fuel display and no browser errors. Parent reports 25 tests/build/schema/396-cell checks passing. Only II/35 reversing west was recorded/inspected in `batch06-movement.mp4`; the other three use DOM/screenshots. Foot figures translate standing poses, not walking cycles. The II/35 footage exposed frame9 as rear-left rather than west; its correction and repeat check are described below. All four now have scoped `gameplay_verified: true`; II/35 preserves prior-revision evidence separately.

## Seventh unique-art batch and west-facing correction

Three new identities have separate artwork:

- `unit-italian-ariete-132nd-div-hq-engineering`: cream open staff car with brown luggage, green roll and aerial, identity citation p.147. Frames 14–17 were regenerated for right-profile closure. This represents the existing HQ counter, not a newly invented engineer battalion or sourced vehicle assignment.
- `unit-italian-16th-engineer-bn`: apron-wearing figure with toolbox, shovel and beige bedroll, identity citation p.149. A brown-background/wrong-start sheet was rejected; the transparent replacement received four corrected closing views.
- `unit-german-33rd-pioneer-bn`: helmeted engineer with shoulder pickaxe, rope and measuring stick, identity citation p.163. Original frames 7/8 were swapped to reduce a small reversal.

Light/dark checks confirm upright broad turning coverage, complete silhouettes and clean cell margins. Near repeats remain. The 16th Engineer also has apparent tool-hand inconsistency around frames 2–3; Ariete has local angle inconsistencies around 6–8. These limitations are not hidden by artwork availability or treated as accurate 18-heading coverage.

**II/35 west correction:** three freshly generated views replace frames 7–9. Inspected frame9 now has nose/toes pointing left and a narrow side backpack, rather than the former broad rear-facing pack. Portrait frame7 changes with it. No flat rotation or mirroring was used. A first corrective strip had right-facing poses and was rejected. Abrupt transitions at 9–10 and 16–17 and other repeated spans remain; this is a bounded correction, not full angular calibration.

Parent reports batch07 searched deployment and unique paths for the new three plus revised II/35, then 11,7 → 12,7 movement through 58–60 sampled positions and 19–21 fractional blends, retained nodes, no rotate/rest blend, 1 CP and no applicable fuel display. Explicit II/35 west reversal passed 67 sampled positions, final frame9 `viewBox="768 256 256 256"`, destination `translate(872 478.45)` and zero blend. Parent inspected before/after frame9, screenshot and footage confirming left-facing nose/toes and narrow backpack. `batch07-west-movement.mp4` covers only II/35; new three use DOM/screenshots (`batch07-unit-*.jpg`). Parent reports 25 tests/build/schema/450-cell checks passing, no browser errors and unchanged source catalog. Scoped `gameplay_verified` is true for these four. These checks are parent-reported, not independently repeated here or exhaustive headings, smoothness or historical certification.

## Eighth unique-art batch

Four HQ identities have separate illustrative portraits and 18-view atlases:

- `unit-commonwealth-2nd-armored-div-hq`: sand staff car with rear half-roof, spare wheel and red pennant, identity citation p.117. Frames 14–17 were regenerated to return toward right profile.
- `unit-commonwealth-1st-armored-bde-hq`: map-reading officer with service cap, binocular case and green radio pack, identity citation p.117.
- `unit-commonwealth-3rd-armored-bde-hq`: side-capped signaller in shorts with blue-gray radio pack, handset and notebook, identity citation p.117.
- `unit-german-15th-panzer-div-hq`: ribbed-door open command car with nose spare wheel and rear radio case, identity citation p.163. Six new views replace frames 3–5 and 12–14, removing an incorrectly front-facing original frame12.

All four light/dark proofs were inspected after packing: upright broad right/front/left/rear/right order, no clipping, obvious halos or neighboring-cell bleed. The two figures have left-facing frame9. Near-repeat spans remain substantial, and nominal 20-degree headings are not calibrated. The Commonwealth car's corrective views are brighter/sharper and frame17 widens versus16 and differs from0. The map officer's wrap changes chest/pack visibility, map and stance. Per-entry limitations record these rather than implying consistent 3D geometry or walking cycles.

These are independently generated illustrations, not sourced historical uniforms, vehicle assignments, paint or insignia. No source rows, aliases, stats or application code changed. A Queen Elizabeth naval experiment was rejected for wrong geometry/headings and a 19-cell layout; it remains missing, not counted as ready. The prior 76th Anti-tank generation rejection remains unresolved.

Parent reports all four searched Player-controlled deployments resolving unique paths, then 11,7 → 12,7 movement with 60 positions each, 19–20 fractional blends, retained nodes, zero rotate/rest blend and 1 CP cost. Fuel remains inapplicable under existing HQ sandbox capabilities, including the car illustrations. Parent reports 25 tests/build/schema/522-cell checks passing, unchanged source catalog and no browser errors. Parent inspected four gameplay screenshots (`batch08-unit-*.jpg`), and recorded/inspected only 15th Panzer HQ in `batch08-movement.mp4` plus a 5fps sheet. Other three use DOM/screenshots. Scoped `gameplay_verified` is true. These are parent-reported bounded checks, not independent repetition or all-angle/historical certification.

## Ninth unique-art batch

Four Commonwealth tank identities from physical scan p.117 have unique illustrative sets:

- `unit-commonwealth-3rd-royal-tank-regt`: sand-gray tank with yellow pennant, brown rear cases and blue-gray turret blanket. Six new rear views replace frames 10–15, removing wrongly front-facing original frames 10–11.
- `unit-commonwealth-5th-royal-tank-regt`: sand/rust-red tank with turret spare wheel, green rear bedroll and cream crate. New strip views 0,3,4,5 replace frames 0,15,16,17; unused strip2 has gun/hull disagreement. Original13 remains after the replacement was visibly undersized.
- `unit-commonwealth-6th-royal-tank-regt`: sand tank with blue-gray stripe, rear tan roll and brown locker. Three new closing views replace frames 15–17; original14 remains.
- `unit-commonwealth-7th-queens-own-hussars`: sand/sage tank with raised cupola, dark rear rolls and ivory turret pouch. Three new closing views replace frames 15–17; original14 remains.

Initial packing oversized the last three views for 6th RTR and 7th Hussars. Matching corrective-strip profile width to the original profile corrected the obvious jump; final light/dark proofs were reinspected. 5th RTR's new right profile is likewise normalized against its original opposite profile. All four retain clean margins, upright broad turning coverage and no gross hull/gun reversals. Near repeats, uneven yaw and detail/proportion discontinuities remained, particularly 5th RTR's 14–15 height change and several loop boundaries. These are not calibrated 20-degree models, exact vehicle reconstructions or historically sourced paint/stowage. No identity, alias, source-stat or gameplay changes.

Parent reports batch09 searched deployments and distinct asset paths for all four, movement 11,7 → 12,7 with 59–60 sampled positions, 20–21 blends, retained nodes, zero rotate/rest blend, 1 CP/8 fuel cost and no browser errors. Parent reports 25 tests/build/schema/594-cell checks passing, unchanged source catalog, and inspected four screenshots (`batch09-unit-*.jpg`) and only 7th Hussars footage (`batch09-movement.mp4` plus a 5fps sheet). The other three use DOM/screenshots. Scoped `gameplay_verified` is true for unchanged 6th RTR and 7th Hussars. Prior-revision evidence is preserved for 3rd/5th RTR, whose new repairs below await repeat checks. These are parent-reported bounded checks, not exhaustive art certification.

## Tenth batch: tank transition repairs

Parent's atlas inspection identified the 5th RTR height drop and 3rd RTR's closing silhouette inconsistency; those original findings were not from 3rd/5th gameplay footage. Each now receives four freshly generated views, referenced to its packed frame14, replacing only frames 15,16,17,0. All other cells and portraits remain byte-identical. No flat rotation, mirroring or stretching was used.

- **5th RTR:** frame14/15 alpha-bounds heights change from 156/131px to 156/156px. The before/after comparison visibly reduces the drop and preserves a taller turret/hull. It does not remove the width/viewpoint/texture change at 14–15, shorter frame17, or wrap/0–1 size and style pops. Single-image generation/edit attempts returned studio backgrounds and were rejected; the accepted four-view strip has inspected transparency.
- **3rd RTR:** the generated closing views keep the rear blanket, cases and turret silhouette more consistent. Frame14/15 alpha heights are now 164/159px (previously 164/161px); the improvement is qualitative shape consistency, not a numeric height fix. Width/viewpoint change, flag/running-gear details and wrap/0–1 proportion discontinuities remain.

The labeled `batch10-tank-transition-comparison.png` and both final light/dark sheets were inspected. Broad upright yaw and clean margins remain; near-repeat spans and large angle steps persist. **Neither repair is a smoothness or accurate-heading certification.** At that delivery counts remained 33 sets available and 1,323 missing among 1,356 provisional candidates. Source catalog and identity mappings were unchanged.

Parent independently confirmed only cells0/15/16/17 changed and portraits remained byte-identical. Parent reports deliberate browser movement 11,7 → 11,6 → 12,6 through frames12–17→0 for both revisions, with blended14→15, 46 distinct positions for 3rd and 44 for 5th, retained nodes and restblend0. Each leg costs1CP/8fuel; unique paths resolve and browser errors are empty. Parent reports 25 tests/build/schema/594-cell checks passing and source catalog unchanged. Parent inspected `batch10-{3rd,5th}.jpg` and only 5th footage (`batch10-5th-transition.mp4` plus10fps sheet); 3rd's encoder failed, so its evidence is DOM/screenshots. Scoped `gameplay_verified` is true for the revisions. Parent still observes14→15 widening and17→0→1 size/style pops. These bounded parent-reported checks do not certify smoothness or every heading.

## Eleventh unique-art batch

Four additional Commonwealth infantry identities on physical scan p.117 receive independently generated upright sets:

- `unit-commonwealth-1st-tower-hamlet-rifles`: tan helmet, square tan pack and blue-gray top roll. Three generated leftward replacements occupy7–9.
- `unit-commonwealth-1st-rangers`: sidecap, shorts, green pack and lower tan roll. Original generated18-view sheet retained.
- `unit-commonwealth-1st-krrc`: dark helmet with tan band, brown pack and vertical pale roll. Reviewed6–9 order is corrective0, corrective1, original6, corrective2.
- `unit-commonwealth-2nd-rifle-brigade`: broad-brimmed hat, shorts, olive pack, dark top roll and middle cream blanket. Only corrective strip2 replaces9; the other two corrective views faced right and were rejected.

Final light/dark proofs were inspected for upright broad yaw, transparent edges, complete silhouettes, clear west-facing frame9 and cell separation. All retain substantial near-repeat spans, uneven angular steps, stance and gear changes. Rangers12–14 mostly changes head orientation rather than body yaw; KRRC7–8 has slight hesitation. The9 profile stance narrows abruptly in several sets. These are standing figures translated by the renderer, not walking cycles. Uniforms and equipment distinguish illustrations but are not sourced historical assignments. No calibrated18-heading, smooth3D or exact-model claim is made.

At delivery counts were37 sets available/1,319 missing among1,356 provisional candidates. Source rows, aliases, stats, canonical IDs and application code were unchanged.

Parent reports searched Player-controlled deployment/unique paths for all four and11,7→12,7 movement with59–60 positions/19–20 blends, retained nodes, zero rotation/rest blend,1CP and fuel not applicable. Parent reports25tests/build/schema/666cells passing, empty browser errors and unchanged source catalog. Parent inspected light/dark proofs, zero-based frame9 crops (all left-facing), and four `batch11-unit-*.jpg` screenshots. Only2nd Rifle Brigade has recorded/inspected west-return footage (`batch11-movement.mp4` plus5fps sheet), ending viewBox768 256 256 256/restblend0; other three use DOM/screenshots. Scoped `gameplay_verified` is true. These are bounded parent-reported checks, not exhaustive headings, walking cycles or historical certification.

## Twelfth unique-art batch

Four new Commonwealth identities have independent Painter illustrations:

- `unit-commonwealth-11th-royal-horse-arty` (scan116): sand field gun with blue-gray trail tarp and brown box. The generated sheet has19 views, with7 in the bottom row. Original12 nearly repeats11 and is excluded; the remaining18 are packed into the standard grid. Near repeats2–3,6–7,12–13 and a conspicuous15→16 profile/height jump remain.
- `unit-commonwealth-2nd-royal-horse-arty` (scan117): sand/sage field gun with cream canvas bundle and dark toolbox. Original18 views retained. Near repeats1–2 and10–11, larger0→1/4→5/9→10 steps and17→0 perspective change remain.
- `unit-commonwealth-32nd-army-tank-bde-hq` (scan117): sun-helmeted map officer in shorts with olive pack/cream roll. Corrective front-left views and reordering replace the original rear-left reversal. Final6–9 are corrective0, corrective1, original9, corrective2. Near repeats, narrower stance6–9 and substantial17→0 turn remain.
- `unit-commonwealth-2nd-support-group-hq` (scan117): side-capped wireless operator with long trousers, olive radio pack and handset. Original sheet started front and reversed its rear arc. Six new rightward views plus explicit original-view reordering restore broad clockwise coverage. Near repeats and larger12–14 steps, arm/gear/foot changes remain.

Final light/dark proofs were inspected after packing/reordering: upright broad right/front/left/rear/right coverage, complete silhouettes, clean margins and no obvious halos or cell bleed. Files retain18cells,6×3,256px, x128/y240 alignment and portrait=frame7; no flat image rotation, reflection or recolored-role substitutions. **Eighteen stored views do not establish eighteen accurately spaced directions.** These are illustrative guns/uniforms/stowage, not exact equipment reconstructions or historical unit markings. Guns translate without crew/towing animation and people remain standing.

A61st Light AA candidate was rejected: its upper assembly/barrel changes while the carriage remains in a similar projection, with repeated orientations rather than reliable full-circle yaw. It stays `missing`, null paths; rejected provenance is recorded. Its existing `heavy-anti-air` role is a simplified capability family, not proof that this light regiment had heavy guns. The76th Anti-tank generation rejection and rejected naval experiment also remain unresolved.

At delivery coverage was 41 sets available / 1,315 missing of 1,356 provisional candidates. Canonical IDs, source rows/aliases/stats and source catalog hash were unchanged.

Parent reports all four searched Player-controlled placements resolving unique portrait/atlas paths, followed by 11,7 → 12,7 movement with 59–60 sampled positions, 20–22 blends, retained nodes, zero rotate/rest blend and 1 CP each. Guns spend 8 fuel; HQ fuel is inapplicable. Parent reports 25 tests/build/schema/738-cell checks passing, browser errors empty and source catalog unchanged. Parent inspected all four `batch12-unit-*.jpg` screenshots and light/dark proofs. Only 11th RHA has footage: the additional 11,7 → 11,6 → 12,6 path crossed frames12–17→0 with 43 positions and blended15→16, recorded/inspected in `batch12-artillery-turn.mp4` plus a 10fps sheet. Other three use DOM/screenshots. The video is too small to certify fine geometry; atlas inspection confirms the 15→16 height/profile jump. Scoped `gameplay_verified` is true for the unchanged three, while this prior-revision evidence is preserved separately for the revised 11th RHA below.

## Thirteenth batch: two identities and an artillery repair

Two new identities on physical scan p.117 have independently generated sets:

- `unit-commonwealth-kings-dragoon-guards`: sand armored car with olive side panel, rear spare tire and blue-gray fender tarp. The original 18-view generation provides broad clockwise coverage. Near repeats at1–2/6–7/10–11, larger3→4/12→13 steps, wheel-spacing changes and the17→0 perspective change remain.
- `unit-commonwealth-1st-royal-tank-regt`: sand cruiser tank with cream turret band, gray rear turret box and green bedrolls. The initial sheet lacked the right-front arc; six freshly generated rightward views and explicit original-view reordering supply it. Corrective views use the opposite profile's width for scale. Final proofs show broad clockwise coverage, but3→4 narrows abruptly,15→16 changes barrel projection, and turret/hull yaw does not always advance together. Near repeats and the loop seam remain.

For **11th RHA**, two new rear-right views replace only zero-based16 and17. Frame15→16 alpha-bounds heights change from149→100px to149→134px, reducing the immediate height drop from49px to15px and keeping16 less abruptly side-on. This is a local improvement with tradeoffs:16→17 still drops134→100px, and17→0 now has a larger barrel-angle change than before. Wheel, shield and texture differences remain. All other cells and the portrait are byte-identical. No flat rotation, mirroring or nonuniform stretching was used.

The labeled before/after comparison and all three final light/dark proofs were inspected: upright broad turning order, complete silhouettes, clean margins and no obvious halos or cell bleed. These static reviews do not establish smooth motion or calibrated20-degree headings. New illustrations are not historical equipment/marking certifications.

Parent independently confirmed only RHA16/17 changed and the portrait remained byte-identical. Parent reports KDG and 1st RTR searched deployment/unique paths and 11,7 → 12,7 movement passing with 59 positions/20 blends each, retained nodes, zero rotation/rest blend and 1 CP/8 fuel. Revised RHA crossed12–17→0 along11,7 → 11,6 → 12,6 with46 positions, blended15→16, retained node/restblend0 and1CP/8fuel per leg. Parent reports25tests/build/schema/774-cell checks passing, empty browser errors and unchanged source catalog. Parent inspected three gameplay screenshots; new two have DOM/screenshots only. The first30fps RHA encoder failed, but the repeated move recorded successfully at15fps in `batch13-artillery-turn.mp4`, inspected with a10fps sheet. Scoped `gameplay_verified` is true for all three. Persistent16→17/17→0 jumps and other limitations remain; these are bounded parent-reported checks, not smoothness or all-heading certification.

At delivery coverage was **43 sets available / 1,313 missing among 1,356 provisional candidates**. Source catalog, canonical identities, mappings, stats and renderer contract were unchanged.

## Fourteenth unique-art batch

Three further Commonwealth identities from physical scan p.117 receive independent Painter-generated sets:

- `unit-commonwealth-8th-hussars`: sand cruiser tank with angular open-cupola turret, sage sidestripe, tan rear locker and blue-gray bedroll. Near repeats2–3/6–7/10–11 and larger3→4/8→9/11→12 steps remain. The antenna has low contrast on dark backgrounds.
- `unit-commonwealth-11th-hussars`: pale sand armored car with olive roll, brown canister and low rear spare wheel. Near repeats2–3/6–7/10–11 remain. At3–4 the barrel projects right as the hull approaches front; alignment is imperfect. The side9 silhouette is flatter/longer than adjacent quarter views.
- `unit-commonwealth-4th-ryl-horse-arty`: tan field gun with green trail bundle and pale wooden crate. Near repeats1–3/5–7/10–11 and larger0→1/3→4/9→10/12→13 steps remain. Shield and wheel exposure vary through15→17→0; rigid geometry is not established.

All three retain their original18 generated views; no synthetic rotations, reflections or recolored-role substitutions. Final light/dark proofs were inspected for broad upright clockwise yaw, complete silhouettes, margins, halos and cell bleed. They support broad turning coverage, not measured20-degree increments or smooth animation. Source citations identify units, not the invented paint, stowage or exact equipment model illustrated. The gun has no crew/towing cycle. Paths/atlas geometry/portrait-frame contract remain unchanged.

Parent reports all three searched Player-controlled deployments with unique portrait/atlas paths and11,7 → 12,7 movement:60 positions each,20–21 blends, retained nodes, zero rotation/rest blend and1CP/8fuel. Parent reports25tests/build/schema/828-cell checks passing, empty browser errors and unchanged source catalog. Parent inspected light/dark proofs and all three `batch14-unit-*.jpg` screenshots. Only4th RHA's west return has recorded/inspected footage, at30fps in `batch14-artillery-movement.mp4` with a5fps sheet;8th/11th Hussars use DOM/screenshots. Scoped `gameplay_verified` is true. These bounded checks preserve the repeated-angle/alignment/geometry limitations and do not certify smoothness, precise yaw or towing animation.

At delivery coverage was **46 sets available / 1,310 missing among 1,356 provisional candidates**. Canonical identities, source catalog, mappings and stats were unchanged.

## Fifteenth unique-art batch

Three headquarters identities from physical scan p.117 receive independent illustrative tank sets:

- `unit-commonwealth-7th-armored-div-hq-engineering`: compact command tank with olive radio box, cream roll and short paired barrels.
- `unit-commonwealth-4th-armored-bde-hq`: sand/ochre command tank with brown rear case, green roll and short paired barrels.
- `unit-commonwealth-7th-armored-bde-hq`: cruiser command tank with two small front turret-like structures, maroon turret-side panel, pale roll and dark rear crate.

The supplied page image was rechecked: the first two rows list **“1 × Mark VI Light”**, and7th Armored Brigade HQ lists **“1 × A9”**. These are printed TOE strength entries, not literal vehicle counts. They motivate representative tank illustrations, but the generated hulls, running gear, turrets and small fittings are **not verified Mark VI/A9 reconstructions**. Paint, markings and stowage are artistic differentiation rather than sourced assignments.

All three final light/dark proofs were inspected: broad upright clockwise right/front/left/rear/right coverage and complete silhouettes without obvious clipping, pale halos or cell bleed. Substantial near-repeat spans remain, especially1–3/6–8/10–12; angular steps, wheel spacing, turret proportions and stowage change. Antennas lose contrast on dark backgrounds. Original18 views are retained with the same alpha extraction/scale/anchor and6×3/256px contract. No flat rotations, reflections or shared-role recolors were used.

**These remain `headquarters` roles.** Tank artwork does not grant tank capabilities, change sandbox fuel behavior, or map the original seven prototype formations to these HQ identities. No stats, aliases, canonical IDs or application code changed. None is calibrated-yaw or smooth-turn certified.

**Parent-reported batch15 verification:** all three passed searched placement, canonical portrait/atlas resolution and11,7→12,7 movement with59–60 distinct positions/19–22 blended samples, retained nodes, zero rotations/rest blend andCP3→2. All remain Headquarters/range0/Rally/fuel “—”; original seven remain shared proxies. Parent inspected three screenshots and7th Armored Bde HQ return-movement recording/contact sheet at `.amp/in/artifacts/batch15-hq-movement.mp4`; footage covers only that identity, others have DOM/screenshots. Schema/882-cell checks,25tests, build and diff-check passed; source hash unchanged. This bounded evidence does not establish calibrated yaw or historical geometry. Near repeats and detail pops remain.

Coverage is **49 sets available / 1,307 missing among 1,356 provisional candidates**. Source catalog and identity mappings remain unchanged. Full unique-art coverage and identity reconciliation remain unfinished; work is local, uncommitted and unpushed.

## Sixteenth unique-art batch

Three more identities from physical scan p.117 receive independent Painter sets:

- `unit-commonwealth-7th-support-group-hq`: map officer, dark sidecap, round olive pack, cream roll and reddish brown map case.
- `unit-commonwealth-7th-motor-bde-hq`: radio operator, sun helmet, olive shirt, shorts and square radio pack with cream straps.
- `unit-commonwealth-3rd-ryl-horse-arty`: compact sand anti-tank gun, narrow barrel, low shield, dark wheel hubs, three-foot support and blue-gray breech canvas.

The supplied scan was rechecked: both HQ equipment entries are **“N”**;3rd RHA lists **“6 × 2-pounders”**. These are TOE entries, not uniform references or literal weapon counts. The HQ characters are artistic representations. The gun is representative equipment, not a historically verified reconstruction. Roles remain `headquarters`, `headquarters` and `anti-tank`; no stats or identity mappings changed.

The first gun generation was rejected because it copied too much of the field-gun reference's geometry and stowage. A fresh generation without an image reference produced the compact support-mounted gun. All final sheets preserve18 original generated views; alpha extraction, common scale, center x128/bottom y240,6×3/256px cells and portrait frame7 remain unchanged. No flat rotation, mirroring or shared-role recoloring was used.

All three full light/dark proofs were inspected. They show upright broad clockwise right0/front4–5/left9/rear13/right17 coverage, complete silhouettes, no obvious clipping, broad halos or cell bleed. Neighboring angles remain near repeats: map officer1–2/4–5/6–8/14–15; radio operator1–3/4–5/6–7/10–12/14–16; gun6–7/13–14. Larger angular jumps and fine detail changes remain. The gun's similarly front-splayed support feet do not establish whole-base co-rotation; fine light edging remains on some shield/base contours. Figures translate standing poses; gun has no crew or towing cycle. **Asset availability is not smoothness, calibrated18-heading or historical fidelity certification.**

**Parent-reported batch16 verification and motion change:** after rebuilding the embedded inventory, all three searched deployments resolved canonical portrait/atlas paths. At Kevin's request, the parent changed presentation to **finish turning in place, then travel at fixed heading**. Browser samples showed 56–58 stationary turning frames followed by 27–28 distinct positions, one heading and no blend during travel, retained nodes, no sprite rotation and zero rest blend. Parent inspected all three `.amp/in/artifacts/batch16-unit-*.jpg` screenshots; only 3rd RHA has inspected recording/contact-sheet evidence at `.amp/in/artifacts/batch16-turn-then-move.mp4`. Schema/936-cell checks, 25 tests, build, Clippy, formatting, diff-check and reduced-motion snap passed; browser errors empty. Roles, source mappings and catalog hash unchanged. These are bounded checks, not calibrated yaw or historical geometry certification. Application motion code belongs to the parent; this artwork handoff does not contain `motion.rs` or README changes.

Coverage is **52 sets available / 1,304 missing among 1,356 provisional candidates**. Full unique-art coverage, equipment aliases and counter reconciliation remain unfinished. Source catalog unchanged; local, uncommitted and unpushed.

## Seventeenth unique-art batch

Three identities from physical scan p.118 receive independent Painter sets:

- `unit-commonwealth-40th-royal-tank-regt`: representative sand infantry tank with olive turret panel, charcoal rear roll and two tan boxes.
- `unit-commonwealth-7th-rifle-brigade`: helmeted rifleman with cream scarf, tan backpack and lower green roll.
- `unit-commonwealth-11th-krrc`: side-capped rifleman in shorts with square gray-green pack, vertical pale blanket and brown haversack.

The supplied scan was rechecked. 40th RTR lists **“1 × Matilda, 9 × Valentine”** as TOE strength, not literal vehicle counts. Both infantry equipment entries are **“N”**; footnote a expands **“11th Kings Royal Rifle Corps.”** Tank geometry and all clothing/markings are illustrative. The existing `crusader` role is a sandbox capability mapping, **not a claim that 40th RTR used Crusaders**. No roles, stats, canonical IDs or source mappings changed.

The original tank sheet's front arc had a sideways barrel and unwanted white flash. New views replace zero-based 3–6; the corrective strip's fifth view was rejected for a rear-quarter hull inside the front-left arc. A separately generated front-left view replaces 7 and was re-inspected: 6→7→8 now stays front-left with no gross reversal or scale outlier. Each infantry set receives three corrective leftward views at 7–9. Processing retains real generated perspectives, alpha extraction, common final scale, center x128/bottom y240, 18 cells in a 6×3 grid, 256px cells and portrait from frame7. No flat rotation, mirroring or shared-role recoloring.

All three final light/dark proofs were inspected: complete upright silhouettes, broad directional anchors, no obvious clipping, broad halos or cell bleed. Tank 4–5 remain near-identical head-on poses; rear spans repeat and 16→17 jumps to profile. 7th Rifle Brigade has a slight apparent rear-angle backtrack at 12–14 and stance changes; 11th KRRC repeats rear poses at 12–14 and narrows its stance through 7–9 before widening at 10. Gear, wheel and lighting details vary. **No calibrated-yaw, smoothness, exact historical equipment or walking-cycle certification.** These three await parent gameplay checks under turn-before-travel presentation.

Coverage is **55 sets available / 1,301 missing among 1,356 provisional candidates**. Full coverage and identity reconciliation remain unfinished. Source catalog unchanged; local, uncommitted and unpushed.

## Validation

```sh
uvx check-jsonschema --schemafile docs/unit-identities.schema.json public/data/unit-identities.json
```

Additional delivery checks cover unique IDs, all 1,398 source-row mappings, redirect targets, role IDs, source references, inventory/coverage counts, prototype isolation, asset existence, RGBA dimensions/alpha, distinct cell pixels, transparent margins, portrait/frame agreement and unique files across identities. Mutation controls reject dangling mappings, use of a shared role path as unique art, missing ready assets and opaque cell backgrounds. Light/dark contact sheets were visually inspected after rear-strip replacement; no gross heading reversals, clipping, neighbor fragments or opaque rectangles were visible. Similar neighboring angles remain a documented limitation, not a passed smoothness claim.

The original source catalog and old shared assets are unchanged. No authored gameplay, frontend, animation or browser-build code belongs to this handoff.
