use super::{escape, Catalog, Stat};
use cna_catalog::catalog::FormationReference;

impl Catalog {
    // Arrival quantities and totals carry both a resolved number and a printed
    // code or calculation. Keep them separate (e.g. 2 TOE, source code N).
    pub(super) fn reference_value(&self, stat: &Stat) -> String {
        let value = match (stat.status.as_str(), stat.value) {
            ("sourced", Some(value)) => format!("<strong>{value}</strong>"),
            ("not_applicable", _) => "Not applicable".into(),
            ("conflict", _) => "Conflicting sources — value withheld".into(),
            ("provisional", _) => "Provisional — value withheld".into(),
            ("unverified", _) => "Unverified — value withheld".into(),
            _ => "Unresolved — see source note".into(),
        };
        let raw = stat
            .raw
            .as_ref()
            .map(|raw| format!("<small>Source / calculation: {}</small>", escape(raw)))
            .unwrap_or_default();
        format!(
            "{value}<small>{}</small>{raw}{}",
            escape(stat.unit.as_deref().unwrap_or("")),
            if stat.conditional {
                "<small><b>Conditional — see note</b></small>"
            } else {
                ""
            }
        )
    }

    fn reference_fact(&self, name: &str, stat: &Stat) -> String {
        format!(
            "<div><h5>{}</h5>{}<p>{}</p><small>{}</small></div>",
            escape(name),
            self.reference_value(stat),
            escape(&stat.note),
            self.citations(&stat.source_refs)
        )
    }

    pub(super) fn formation_reference(
        &self,
        reference: &FormationReference,
        tokens: &str,
    ) -> String {
        let cpa = reference
            .base_stats
            .get("capability_points")
            .map(|stat| {
                format!(
                    "{}<p>{}</p><small>{}</small>",
                    self.stat_value("capability_points", stat),
                    escape(&stat.note),
                    self.citations(&stat.source_refs)
                )
            })
            .unwrap_or_else(|| {
                "<p>No sourced base CPA is available; see the profile and arrival notes.</p>".into()
            });
        let mismatch = if reference.base_profile_id != reference.catalog_profile_id {
            format!("<p>Arrival reference profile: {}. Catalog profile remains {}. This is not a change to the unit’s current state.</p>",
                escape(reference.base_profile_id.as_deref().unwrap_or("unresolved")),
                escape(reference.catalog_profile_id.as_deref().unwrap_or("unresolved")))
        } else {
            String::new()
        };
        let base_sources = reference
            .base_stats
            .values()
            .flat_map(|stat| stat.source_refs.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let base = format!(
            "<section class=\"catalog-profile-stats\"><div class=\"catalog-base-cpa\"><h4>Base capability point allowance</h4>{cpa}<p>Base reference only — not current effective CPA.</p></div><p><strong>{}</strong> · Profile {}</p><p>Printed profile code: {}</p><p>{}</p>{mismatch}<details><summary>Base profile characteristics and sources</summary>{}{}</details></section>",
            escape(reference.base_profile_name.as_deref().unwrap_or("No resolved base profile")),
            escape(reference.base_profile_id.as_deref().unwrap_or("unresolved")),
            escape(reference.printed_profile_code.as_deref().unwrap_or("not supplied")),
            escape(&reference.base_profile_note),
            self.characteristics(&reference.base_stats, "Base profile characteristics", false),
            self.source_details(&base_sources));
        let arrival = &reference.arrival;
        let related = arrival.related_record_id.as_ref().map(|id| format!(
            "<p>Related source row: <code>{}</code>. Not an additional assignment; do not add this row to its related formation.</p>", escape(id))).unwrap_or_default();
        let overrides = if arrival.stats.is_empty() {
            String::new()
        } else {
            self.characteristics(
                &arrival.stats,
                "Arrival-specific characteristics — restrictions apply",
                true,
            )
        };
        let components: String = reference.components.iter().map(|component| {
            let ratings = if component.kind == "profile" && component.record_id == reference.base_profile_id {
                "<p>Uses the base profile characteristics shown above; CPA, armor and vulnerability are not multiplied by TOE.</p>".into()
            } else {
                self.characteristics(&component.per_toe_stats, "Component chart characteristics — not multiplied by TOE", false)
            };
            let barrage = component.land_barrage_toe.map(|toe| format!("<p>Land-barrage calculation uses {toe} TOE points under the cited restrictions.</p>")).unwrap_or_default();
            format!("<details class=\"catalog-component\"><summary>{} · printed component {}</summary><p>Record <code>{}</code> · {}</p>{}{barrage}{ratings}</details>",
                escape(&component.name), escape(&component.label), escape(component.record_id.as_deref().unwrap_or("unresolved")),
                escape(&component.kind), self.reference_fact("Assigned arrival TOE", &component.toe))
        }).collect();
        let components = if components.is_empty() {
            "<p>No independent quantified loadout is available. A profile maximum is not an assignment.</p>".into()
        } else {
            components
        };
        format!(
            "<section class=\"catalog-formation-reference\">{base}{tokens}<section class=\"catalog-arrival\"><h4>Historical arrival — not current strength</h4><p>{}</p><div class=\"catalog-arrival-facts\"><div><h5>Printed arrival</h5><strong>{}</strong><p>Stage/turn or source notation; not a current deployment date.</p><h5>Printed TOE / loadout</h5><p>{}</p></div>{}{}</div>{related}{overrides}<details class=\"catalog-citations\"><summary>Arrival sources and row notes</summary>{}</details></section><section class=\"catalog-arrival-totals\"><h4>Arrival combat totals</h4><p>AA uses Actual AA points. Other combat totals are Raw Points before ÷10 and action-level rounding. These supplied reference totals exclude current losses and modifiers.</p>{}</section><section class=\"catalog-arrival-components\"><h4>Arrival components</h4><p>TOE points are not numbers of guns or vehicles. Open a component for its cited quantity and chart ratings.</p>{components}</section><p class=\"catalog-reference-limitations\">{}</p></section>",
            escape(&arrival.note),
            escape(arrival.arrives_raw.as_deref().unwrap_or("Not supplied — see row notes")),
            escape(arrival.toe_raw.as_deref().unwrap_or("Not supplied — see row notes")),
            self.reference_fact("Arrival TOE strength", &arrival.toe_strength),
            self.reference_fact("Arrival morale", &arrival.morale),
            self.source_details(&arrival.source_refs),
            self.characteristics(&reference.arrival_totals, "Sourced arrival calculations", true),
            escape(&reference.limitations))
    }
}
