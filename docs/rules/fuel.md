# Fuel arithmetic: implemented operations and an unresolved source conflict

`official::fuel` computes movement consumption and tank capacity, debits exact
fuel quantities without overdrawing, and applies fuel evaporation/spillage.
These are operations for a trusted rules kernel, **not movement commands or a
complete implementation of section 49**. Nothing is wired into the browser yet.

## ADJ-FUEL-BASIS: pending Kevin

Both conflicting readings exist in the **original** sources, not just VASSAL:

| Evidence | Observable result for one TOE, consumption rate 4 |
| --- | --- |
| R79 physical p67 and AL21 p21, §49.13: “for every five Capability Points (or fraction thereof)”; printed example: 12 CP “would consume 12 Fuel Points” | 1 CP costs 4 fuel; 6 CP costs 8; 12 CP costs 12. |
| Original R79 physical p106, §49.19 chart, visually inspected; VASSAL `49.19 Fuel Consumption Chart.png` matches | Rate-4 column, CP rows 1–5: **0.8, 1.6, 2.4, 3.2, 4**. Proportional accounting gives 12 CP a cost of 9.6. |

`FuelBasis::StartedFiveCpGroups` and `FuelBasis::ProportionalCp` implement these
two readings explicitly. There is **no default**. The host must bind a chosen
basis to the ruleset digest, not let players choose a cheaper basis per order.
Neither option is recorded as approved. Recommendation: prefer §49.13's explicit
prose and worked example; retain the chart discrepancy in the historical record.

The original and July §49.14 capacity formula is CPA × rate ÷ 5 per TOE.
`tank_capacity` implements that literal formula without rounding or filling a
tank. For CPA not divisible by five, its “exactly sufficient” description does
not agree with started-group consumption. This is another consequence of the
same unresolved basis decision, not a reason to silently change the formula.

## Evidence and calculations

- R79 p67 and AL21 p21 §49.13: land combat and other actions with no actual
  hex-to-hex movement consume no fuel. Rates are per TOE, not counter count.
  The caller supplies the reviewed rate and affected TOE; this module does not
  invent values for unresolved equipment or handle the separately abstracted
  aircraft fuel rules.
- §49.15/.16 require fuel in the consuming unit's hex and disallow direct use
  from second-/third-line trucks until off-loaded. The future movement/supply
  kernel must enforce those restrictions before calling `draw` on its private
  transaction copy. `draw` rejects insufficient or unrepresentable stock changes
  without mutation; it is not an authorization/access check.
- R79 p67 and AL21 p21 §49.3: weekly on-map loss is 6%, or 9% for Commonwealth
  through August 1941 inclusive (GT46); 6% from September I (GT47). Hot weather
  causes an additional 5% on the remaining stock, not the week's initial stock.
  The calculation rounds **the loss** down to whole fuel points and excludes
  sea convoys/off-map stocks. Applying the paragraph's rounding/exposure rule to
  its additional hot-weather loss is the current interpretation, not separate
  evidence of an explicit hot-weather rounding sentence.
- The caller must apply weekly loss once in Stores Expenditure and hot-weather
  loss once at weather determination. This module does not own the phase cursor,
  determine weather or prevent repeated calls. It must not be exposed directly
  as a player “evaporate” command.
- VASSAL `buildFile.xml` registers the §49.19 lookup chart in both chart groups.
  The chart contains no evaporation procedure or capacity rule. The module's
  `incorporated errata.txt`, read in full, has no direct §49 correction; that is
  not evidence that all published addenda were examined.

Fuel transportation, emergency siphoning, capacity/access enforcement, vehicle
exceptions, fuel-loss effects in combat and water's separate exposure rules
remain unimplemented. ADJ-SHIP and ADJ-ERR remain pending. This is arithmetic
coverage, not certification of movement or logistics gameplay.

## Verification

`cargo test --locked -p cna-sim --test rules_s49` checks both source readings,
fractional CP/TOE, zero movement and stationary actions, both sides of five-CP
thresholds, exact debits, insufficient stock, representational overflow, weekly
date boundaries, downward loss rounding and sequential weekly/hot losses.
The initial run failed because the API did not exist. No ignored tests or
claims based on indexed case counts are used.
