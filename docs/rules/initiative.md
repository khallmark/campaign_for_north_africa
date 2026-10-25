# Initiative: executable rules and source comparison

`official::initiative::InitiativeRules::execute` implements §7's weekly contest
and the winner's first/last choice for each Operations Stage. This is an engine
kernel, not a playable scenario or a complete phase engine. Commands execute
through `Session::reduce`; accepted retries preserve their outcome and rejected
commands preserve state, random cursor, receipts and events.

## Evidence

Physical PDF pages, not OCR offsets:

| Source | Finding |
| --- | --- |
| `docs/CNAv2.1.0/Rules_Land_07-2021.pdf`, p14, §§7.11–7.16 | Initiative lasts all three stages of the week; each side rolls one die plus its rating; ties roll again. The winner chooses first or last separately each stage. A/B denote order, not fixed nationalities. Scenarios can predetermine opening initiative. |
| `docs/The Campaign for North Africa.pdf`, p13, §7 | Original wording matches the July procedure (OCR cross-check through the searchable companion and `docs/rules.txt`). |
| Original PDF p97, §7.2 chart | Visually inspected: CW GT1–42=3, GT43–90=4, GT91–111=5. Axis: Rommel on the game-map=6; German land combat units but no Rommel=3; neither=1. The Tripoli/Tunisia holding boxes on Map A do not count. |
| `docs/CNAv2.1.0/images/7.2 Initiative Ratings Chart.png` | Visually inspected against original p97: all ratings, endpoints and the holding-box exclusion match. It corroborates the original chart; it is not the rules authority. |
| `docs/CNAv2.1.0/buildFile.xml`, Initiative setup/marker and §7.2 chart registrations | Manual marker and reference chart; no executable initiative procedure established. |
| `docs/CNAv2.1.0/incorporated errata.txt` | Read in full: no §7 correction in this incorporation list. This does not claim that the list exhausts all addenda. |
| `docs/CNAv2.1.0/Rules_Scenarios_07-2021.pdf`, p5, §60.6 | First scenario group gives Italy GT1, then normal determination from GT2. The integration test supplies that override without claiming to load the scenario's forces. |

§7.13 describes date-dependent ratings; the original chart additionally makes
Axis ratings depend on presence. The kernel follows the chart's explicit cases.
It does not silently replace them with a date-only Axis schedule.

## Trust boundary and remaining integration

The host binds `InitiativeRules.roles` and `opening` to the loaded scenario and
reference digests. Roles classify **catalog IDs**, not names parsed from UI
labels. They are not client payload fields. Missing classifications reject.
Tripoli/Tunisia boxes and unambiguously overseas locations do not count as on-map;
relevant pieces in VASSAL port display holding boxes reject until the importer
resolves their physical position. A hex location is trusted scenario input, not
proof of historical placement or a certified map alias.

The host must dispatch determination in the weekly Initiative Determination
Stage and choices in Initiative Declaration. This kernel enforces one result per
week, one winner-only choice per stage and no ordinary midweek determination;
it does **not** supply the remaining phase cursor or an End Stage command.
An explicit scenario opening may start in a later stage. Stage advances in tests
are trusted test-host operations, not a public skip-turn feature.

`GameState.initiative` retains the week's result and three independent choices.
Absent initiative in older diagnostic state deserializes as `None`; it does not
restore a running session or its receipts. Future-dated declarations fail state
validation. Events remain authoritative/private pending viewer-safe projection.

ADJ-SHIP, ADJ-ERR and the mode matrix remain unresolved. This kernel applies the
shared initiative procedure without certifying any complete mode or scenario.

## Executed behavior tests

`cargo test --locked -p cna-sim --test rules_s07` covers the printed October 1940
example, both sides of every CW rating boundary, all three Axis ratings,
holding-box exclusion, seeded tied rolls followed by rerolling **both** dice,
scenario override expiry, first/last/first choices, incorrect actors, repeat
requests, unresolved references, random exhaustion, strict payloads and state
round trips. Expected seed-zero rolls come from independent ChaCha8 TC1 words,
not a second invocation of the rule being tested. The initial test run failed
because the rule API was absent. The strict-payload test also caught Serde's
unit-variant field acceptance; an empty struct variant now rejects extra fields.
