use crate::view::escape;
use cna_catalog::catalog::{Record, Sprite, Stat};
use std::collections::{BTreeMap, BTreeSet};

mod formation;
mod tokens;

const PAGE_SIZE: usize = 24;

pub struct Catalog(cna_catalog::catalog::Catalog);

impl std::ops::Deref for Catalog {
    type Target = cna_catalog::catalog::Catalog;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn kind_name(kind: &str) -> &str {
    match kind {
        "unit_type" => "Unit type",
        "formation" => "Formation",
        "counter_formation" => "Counter-only formation",
        "equipment" => "Equipment",
        "aircraft_configuration" => "Aircraft configuration",
        "ground_support" => "Special ground support unit",
        "naval_unit" => "Naval unit",
        "support_unit" => "Support unit",
        "profile" => "Characteristic profile",
        _ => kind,
    }
}

fn affiliation_name(value: &str) -> &str {
    match value {
        "allied" => "Allies",
        "axis" => "Axis",
        "commonwealth" => "Commonwealth",
        "german" => "German",
        "italian" => "Italian",
        "united-states" => "United States",
        "" | "unspecified" => "Unspecified",
        _ => value,
    }
}

impl Catalog {
    pub fn apply_identity_artwork(&mut self, inventory: &crate::inventory::Inventory) {
        for record in self.0.entries.iter_mut().chain(&mut self.0.profiles) {
            let (path, note) = inventory
                .source_art(&record.id)
                .unwrap_or((None, "Shared role illustration — not unique unit artwork"));
            record.artwork_note = note.into();
            if let Some(path) = path {
                record.sprite = Some(Sprite {
                    id: String::new(),
                    path: path.into(),
                });
            }
        }
    }

    pub fn parse(json: &str) -> Result<Self, String> {
        cna_catalog::catalog::Catalog::parse(json).map(Self)
    }

    pub fn panel(&self) -> String {
        let gaps: String = self
            .coverage
            .gaps
            .iter()
            .map(|gap| format!("<li>{}</li>", escape(gap)))
            .collect();
        let kinds: String = self
            .entries
            .iter()
            .chain(&self.profiles)
            .filter(|record| record.kind != "unit_type")
            .map(|record| record.kind.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|kind| {
                format!(
                    "<option value=\"{}\">{}</option>",
                    escape(kind),
                    escape(kind_name(kind))
                )
            })
            .collect();
        let unit_types: String = self
            .entries
            .iter()
            .chain(&self.profiles)
            .map(|record| record.unit_type.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|id| {
                format!(
                    "<option value=\"{}\">{}</option>",
                    escape(id),
                    escape(self.unit_type_name(id).unwrap_or(affiliation_name(id)))
                )
            })
            .collect();
        let nationalities: String = self
            .entries
            .iter()
            .chain(&self.profiles)
            .map(|record| record.nationality.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|id| {
                format!(
                    "<option value=\"{}\">{}</option>",
                    escape(id),
                    escape(affiliation_name(id))
                )
            })
            .collect();
        format!(
            r#"<div class="catalog-shell"><header><h2>Unit catalog</h2><nav id="catalog-breadcrumbs" aria-label="Catalog breadcrumb">{}</nav></header><div id="catalog-browser" class="catalog-screen">
            <p>Choose a unit type to explore its units, equipment and characteristic profiles. These are source references, not an operational order of battle.</p>
            <details class="catalog-coverage"><summary>{}</summary><p>{}</p><ul>{gaps}</ul><p>Identity verification does not verify every stat. Only sourced numbers are displayed. PDF citations use one-based physical pages.</p></details>
            <div class="catalog-filters"><label>Search records<input id="catalog-query" type="search" placeholder="Unit type, unit name, nationality or ID" data-catalog-filter autofocus /></label>
            <label>Unit type<select id="catalog-unit-type" data-catalog-filter><option value="">All unit types</option>{unit_types}</select></label>
            <label>Record type<select id="catalog-kind" data-catalog-filter><option value="">All record types</option>{kinds}</select></label>
            <label>Side<select id="catalog-side" data-catalog-filter><option value="">All sides</option><option value="allied">Allies</option><option value="axis">Axis</option><option value="unspecified">Unspecified</option></select></label>
            <label>Nationality<select id="catalog-nationality" data-catalog-filter><option value="">All nationalities</option>{nationalities}</select></label></div>
            <div id="catalog-results">{}</div></div><div id="catalog-detail" class="catalog-screen" hidden></div></div>"#,
            self.breadcrumbs("", None),
            if self.coverage.complete_historical_roster {
                "Coverage and source limitations"
            } else {
                "Roster transcription incomplete — coverage and limitations"
            },
            escape(&self.coverage.summary),
            self.results("", "", "", "", "", 0)
        )
    }

    pub fn results(
        &self,
        query: &str,
        kind: &str,
        side: &str,
        unit_type: &str,
        nationality: &str,
        page: usize,
    ) -> String {
        let query = query.to_lowercase();
        let matches: Vec<_> = self
            .entries
            .iter()
            .chain(&self.profiles)
            .filter(|record| record.kind != "unit_type")
            .filter(|record| {
                let record_side = if record.side.is_empty() {
                    "unspecified"
                } else {
                    &record.side
                };
                let text = format!(
                    "{} {} {} {} {} {}",
                    record.name,
                    record.id,
                    record.nationality,
                    record.unit_type,
                    kind_name(&record.kind),
                    self.unit_type_name(&record.unit_type)
                        .unwrap_or(&record.unit_type)
                )
                .to_lowercase();
                (kind.is_empty() || record.kind == kind)
                    && (side.is_empty() || record_side == side)
                    && (unit_type.is_empty() || record.unit_type == unit_type)
                    && (nationality.is_empty() || record.nationality == nationality)
                    && query.split_whitespace().all(|word| text.contains(word))
            })
            .collect();
        if unit_type.is_empty() {
            return self.type_index(
                &matches,
                &query,
                kind.is_empty() && side.is_empty() && nationality.is_empty(),
            );
        }
        let definition = self.entries.iter()
            .find(|record| record.kind == "unit_type" && record.unit_type == unit_type)
            .map(|record| format!("<details class=\"catalog-type-reference\"><summary>Type definition and sources</summary>{}</details>", self.record(record)))
            .unwrap_or_default();
        let heading = format!(
            r#"<h3 class="catalog-type-heading" tabindex="-1">{}</h3>{definition}"#,
            escape(
                self.unit_type_name(unit_type)
                    .unwrap_or(affiliation_name(unit_type))
            )
        );
        if matches.is_empty() {
            return format!("{heading}<p class=\"catalog-count\" role=\"status\" tabindex=\"-1\">No matching records. Clear search or filters to see all recorded members of this type. Some types have no member records transcribed yet.</p>");
        }
        let page = page.min((matches.len() - 1) / PAGE_SIZE);
        let start = page * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(matches.len());
        let records: String = matches[start..end]
            .iter()
            .map(|record| format!(r#"<button class="catalog-entry catalog-card" id="catalog-record-{}" data-action="catalog-record" data-record="{}">{}</button>"#,
                escape(&record.id), escape(&record.id), self.record_heading(record)))
            .collect();
        let pagination = if matches.len() > PAGE_SIZE {
            format!(
                r#"<nav class="catalog-pages" aria-label="Catalog pages"><button data-action="catalog-page" data-page="{}" {}>Previous</button><span>Page {} of {}</span><button data-action="catalog-page" data-page="{}" {}>Next</button></nav>"#,
                page.saturating_sub(1),
                if page == 0 { "disabled" } else { "" },
                page + 1,
                matches.len().div_ceil(PAGE_SIZE),
                page + 1,
                if end == matches.len() { "disabled" } else { "" }
            )
        } else {
            String::new()
        };
        format!(
            r#"{heading}<p class="catalog-count" role="status" tabindex="-1">Showing {}–{} of {} records. Expand a record for stats and sources.</p><div class="catalog-list">{records}</div>{pagination}"#,
            start + 1,
            end,
            matches.len()
        )
    }

    fn type_index(&self, matches: &[&Record], query: &str, include_empty: bool) -> String {
        let types = self
            .entries
            .iter()
            .chain(&self.profiles)
            .map(|record| record.unit_type.as_str())
            .collect::<BTreeSet<_>>();
        let mut count = 0;
        let rows: String = types.into_iter().filter_map(|id| {
            let name = self.unit_type_name(id).unwrap_or(affiliation_name(id));
            let members = matches.iter().filter(|record| record.unit_type == id).count();
            let type_text = format!("{name} {id}").to_lowercase();
            if members == 0 && !(include_empty && query.split_whitespace().all(|word| type_text.contains(word))) {
                return None;
            }
            count += 1;
            let image = self.entries.iter()
                .find(|record| record.kind == "unit_type" && record.unit_type == id)
                .and_then(|record| record.sprite.as_ref())
                .map(|sprite| format!("<img src=\"{}\" alt=\"\" loading=\"lazy\" />", escape(&sprite.path)))
                .unwrap_or_default();
            Some(format!(r#"<button class="catalog-type" id="catalog-type-{}" data-action="catalog-type" data-type="{}" aria-label="Explore {}">{image}<strong>{}</strong><span>Matching records: {members}</span><span aria-hidden="true">›</span></button>"#, escape(id), escape(id), escape(name), escape(name)))
        }).collect();
        let message = if count == 0 {
            "No matching unit types. Try a shorter search or clear one or more filters.".into()
        } else {
            format!("Unit types: {count}. Choose a type to explore its records; illustrations represent the type, not individual units.")
        };
        format!(
            r#"<p class="catalog-count" role="status" tabindex="-1">{message}</p><div class="catalog-types">{rows}</div>"#
        )
    }

    pub fn select_type(&self, unit_type: &str) {
        use wasm_bindgen::JsCast;
        let doc = crate::document();
        let select = doc
            .get_element_by_id("catalog-unit-type")
            .unwrap()
            .unchecked_into::<web_sys::HtmlSelectElement>();
        let previous = select.value();
        select.set_value(unit_type);
        self.update_results(0, true);
        let focus = if unit_type.is_empty() {
            doc.get_element_by_id(&format!("catalog-type-{previous}"))
                .or_else(|| doc.query_selector(".catalog-count").unwrap())
        } else {
            doc.query_selector(".catalog-type-heading").unwrap()
        };
        if let Some(focus) = focus {
            let _ = focus.unchecked_ref::<web_sys::HtmlElement>().focus();
        }
    }

    fn stat_value(&self, id: &str, stat: &Stat) -> String {
        match (stat.status.as_str(), stat.value) {
            ("sourced", Some(value)) => {
                let printed = stat.raw.clone().unwrap_or_else(|| value.to_string());
                let unit = stat.unit.as_deref().or_else(|| self.stat_definitions.iter().find(|d| d.id == id).map(|d| d.unit.as_str())).unwrap_or("");
                format!("<strong>{}</strong><small>{}</small>{}", escape(&printed), escape(unit), if stat.conditional { "<small><b>Conditional — see note</b></small>" } else { "" })
            },
            ("not_applicable", _) if stat.raw.as_deref() == Some("-") =>
                "<strong>−</strong><small>Source dash</small><small>See this chart’s legend and note; not a calculated unit rating.</small>".into(),
            ("not_applicable", _) => "Not applicable".into(),
            ("provisional", _) => "Provisional<br><small>Value withheld</small>".into(),
            _ => "Unresolved".into(),
        }
    }

    fn characteristics(
        &self,
        stats: &BTreeMap<String, Stat>,
        caption: &str,
        reference: bool,
    ) -> String {
        let mut stats: Vec<_> = stats.iter().collect();
        stats.sort_by_key(|(id, _)| {
            self.stat_definitions
                .iter()
                .position(|definition| definition.id == id.as_str())
                .unwrap_or(usize::MAX)
        });
        let stats: String = stats.into_iter().map(|(id, stat)| {
            let definition = self.stat_definitions.iter().find(|d| d.id == *id);
            let name = definition.map(|d| d.name.as_str()).unwrap_or(id);
            let description = if reference { "" } else { definition.map(|d| d.description.as_str()).unwrap_or("") };
            let value = if reference { self.reference_value(stat) } else { self.stat_value(id, stat) };
            format!("<tr><th scope=\"row\">{}<small>{}</small><small>{}</small><small>{}</small></th><td>{value}</td></tr>", escape(name), escape(description), escape(&stat.note), self.citations(&stat.source_refs))
        }).collect();
        if stats.is_empty() {
            "<p>No numerical characteristics have been recorded for this entry.</p>".into()
        } else {
            format!("<table><caption>{}</caption><thead><tr><th scope=\"col\">Characteristic and source</th><th scope=\"col\">Value</th></tr></thead><tbody>{stats}</tbody></table>", escape(caption))
        }
    }

    fn aa_equipment_references(&self, record: &Record) -> String {
        if !matches!(
            record.kind.as_str(),
            "formation" | "counter_formation" | "profile"
        ) || !matches!(
            record.unit_type.as_str(),
            "anti-air" | "heavy-anti-air" | "anti-tank-anti-air"
        ) {
            return String::new();
        }
        let rows: String = self.entries.iter()
            .filter(|equipment| equipment.kind == "equipment"
                && equipment.nationality == record.nationality
                && matches!(equipment.unit_type.as_str(), "anti-air" | "heavy-anti-air" | "anti-tank-anti-air"))
            .filter_map(|equipment| equipment.stats.get("anti_air").map(|stat| {
                format!("<tr><th scope=\"row\">{}<small>{}</small><small>{}</small></th><td>{}</td></tr>",
                    escape(&equipment.name), escape(&stat.note), self.citations(&stat.source_refs),
                    self.stat_value("anti_air", stat))
            })).collect();
        if rows.is_empty() {
            return String::new();
        }
        format!("<section class=\"catalog-equipment-ratings\"><h4>Reference weapon ratings — not an assigned loadout</h4><p>AA combat ratings come from weapon TOE, not a profile dash. These are same-nationality AA weapon references, not equipment assigned to this formation or a compatibility list. Current TOE and weapon mix are required for a unit total; no current total is calculated here.</p><table><caption>Anti-aircraft equipment ratings</caption><thead><tr><th scope=\"col\">Weapon and source</th><th scope=\"col\">Anti-aircraft rating</th></tr></thead><tbody>{rows}</tbody></table></section>")
    }

    fn record_heading(&self, record: &Record) -> String {
        let identity = match record.verification.as_str() {
            "verified" if record.kind == "profile" => "Profile verified",
            "verified" => "Identity verified",
            "partial" => "Partially verified",
            "unverified_ocr" => "OCR · unverified",
            _ => "Provisional",
        };
        let sprite = record
            .sprite
            .as_ref()
            .map(|sprite| {
                format!(
                    r#"<img src="{}" alt="" loading="lazy" />"#,
                    escape(&sprite.path)
                )
            })
            .unwrap_or_default();
        let affiliations: String = [&record.nationality, &record.side]
            .into_iter()
            .filter(|value| {
                record.kind != "unit_type" || (!value.is_empty() && value.as_str() != "unspecified")
            })
            .map(|value| format!(" · {}", escape(affiliation_name(value))))
            .collect();
        format!(
            r#"{sprite}<span class="catalog-name"><strong>{}</strong><span>{} · {}{affiliations}</span><small>{}</small></span><span class="catalog-status">{identity}</span>"#,
            escape(&record.name),
            escape(
                self.unit_type_name(&record.unit_type)
                    .unwrap_or(affiliation_name(&record.unit_type))
            ),
            escape(kind_name(&record.kind)),
            escape(&record.artwork_note)
        )
    }

    pub fn detail(&self, id: &str) -> Option<String> {
        self.entries
            .iter()
            .chain(&self.profiles)
            .find(|record| record.id == id)
            .map(|record| self.record(record))
    }

    fn record(&self, record: &Record) -> String {
        let tokens = if record.kind == "unit_type" {
            String::new()
        } else {
            format!(
                r#"<section class="catalog-original-tokens" data-record="{}"><h4>Original counter tokens</h4><p>Loading original-token references…</p></section>"#,
                escape(&record.id)
            )
        };
        let equipment = self.aa_equipment_references(record);
        let formation = record
            .formation_reference
            .as_ref()
            .filter(|reference| reference.schema_version == 1);
        let stats = self.characteristics(
            &record.stats,
            "Rulebook characteristics, not tutorial stats",
            false,
        );
        let stats = if formation.is_some() {
            format!("<section class=\"catalog-direct-stats\"><h4>Current / scenario characteristics</h4><p>Current effective CPA is not established by a base profile or arrival reference. Unknown values below do not mean that base CPA is missing.</p>{stats}</section>")
        } else if record.profile_id.is_some() {
            format!("<section class=\"catalog-direct-stats\"><h4>Formation / scenario characteristics</h4>{stats}</section>")
        } else {
            stats
        };
        let profile = record.profile_id.as_ref().filter(|_| formation.is_none()).map(|id| {
            match self.profiles.iter().find(|profile| profile.id == *id) {
                Some(profile) => format!(
                    "<section class=\"catalog-profile-stats\"><h4>Linked profile characteristics</h4><p><strong>{}</strong> · Profile {}</p><p>Base reference characteristics, not current effective CPA or initial formation/scenario state.</p><p>{}</p>{}<details class=\"catalog-citations\"><summary>Profile sources</summary>{}</details></section>",
                    escape(&profile.name), escape(&profile.id), escape(&profile.note),
                    self.characteristics(&profile.stats, "Base profile characteristics", false), self.source_details(&profile.source_refs)
                ),
                None => format!("<p>Characteristic profile {} is unavailable; linked values are unresolved.</p>", escape(id)),
            }
        }).unwrap_or_default();
        let reference = formation
            .map(|reference| self.formation_reference(reference, &tokens))
            .unwrap_or_default();
        let tokens = if formation.is_some() { "" } else { &tokens };
        let note = if formation.is_some() {
            "Historical source row, not a playable deployment. Arrival notes and limitations are shown below."
        } else {
            &record.note
        };
        format!(
            r#"<article class="catalog-entry"><h3 class="catalog-record-heading" tabindex="-1">{}</h3><div class="catalog-record"><p class="catalog-id">Record {}</p><p>{}</p>{reference}{tokens}{equipment}{stats}<details class="catalog-citations"><summary>Record sources</summary>{}</details>{profile}</div></article>"#,
            self.record_heading(record),
            escape(&record.id),
            escape(note),
            self.source_details(&record.source_refs)
        )
    }

    fn citations(&self, refs: &[String]) -> String {
        refs.iter()
            .map(|id| {
                self.sources
                    .iter()
                    .find(|source| source.id == *id)
                    .map(|source| {
                        format!(
                            "{}{}{}",
                            escape(&source.title),
                            source
                                .section
                                .as_ref()
                                .filter(|section| !section.is_empty())
                                .map(|s| format!(" · §{}", escape(s)))
                                .unwrap_or_default(),
                            source
                                .pdf_page
                                .map(|p| format!(" · PDF p. {p}"))
                                .unwrap_or_default()
                        )
                    })
                    .unwrap_or_else(|| escape(id))
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn source_details(&self, refs: &[String]) -> String {
        refs.iter()
            .map(|id| {
                let citation = self.citations(std::slice::from_ref(id));
                self.sources
                    .iter()
                    .find(|source| source.id == *id)
                    .map(|source| {
                        format!(
                            "<p><strong>{citation}</strong><br>{}<br>{}</p>",
                            escape(&source.path),
                            escape(&source.note)
                        )
                    })
                    .unwrap_or_else(|| format!("<p>{citation}</p>"))
            })
            .collect()
    }

    fn breadcrumbs(&self, unit_type: &str, record: Option<&Record>) -> String {
        if unit_type.is_empty() {
            return "<ol><li aria-current=\"page\">Unit types</li></ol>".into();
        }
        let name = escape(
            self.unit_type_name(unit_type)
                .unwrap_or(affiliation_name(unit_type)),
        );
        let tail = match record {
            Some(record) => format!(
                r#"<li><button data-action="catalog-list" aria-label="Back to {name}">{name}</button></li><li aria-current="page">{}</li>"#,
                escape(&record.name)
            ),
            None => format!("<li aria-current=\"page\">{name}</li>"),
        };
        let back = if record.is_some() {
            "data-action=\"catalog-list\""
        } else {
            "data-action=\"catalog-type\" data-type=\"\""
        };
        format!(
            r#"<button class="catalog-back" {back} aria-label="Back">← Back</button><ol><li><button data-action="catalog-type" data-type="" aria-label="Back to unit types">Unit types</button></li>{tail}</ol>"#
        )
    }

    pub fn open_record(&self, id: &str) {
        use wasm_bindgen::JsCast;
        let Some(record) = self
            .entries
            .iter()
            .chain(&self.profiles)
            .find(|record| record.id == id)
        else {
            return;
        };
        let doc = crate::document();
        let detail = doc.get_element_by_id("catalog-detail").unwrap();
        detail.set_inner_html(&self.detail(id).unwrap());
        detail.set_attribute("data-record", id).unwrap();
        doc.get_element_by_id("catalog-browser")
            .unwrap()
            .set_attribute("hidden", "")
            .unwrap();
        detail.remove_attribute("hidden").unwrap();
        detail.set_scroll_top(0);
        doc.get_element_by_id("catalog-breadcrumbs")
            .unwrap()
            .set_inner_html(&self.breadcrumbs(&record.unit_type, Some(record)));
        let _ = detail
            .query_selector(".catalog-record-heading")
            .unwrap()
            .unwrap()
            .unchecked_ref::<web_sys::HtmlElement>()
            .focus();
        tokens::load();
    }

    pub fn back_to_list(&self) {
        use wasm_bindgen::JsCast;
        let doc = crate::document();
        let detail = doc.get_element_by_id("catalog-detail").unwrap();
        let id = detail.get_attribute("data-record").unwrap();
        detail.set_attribute("hidden", "").unwrap();
        doc.get_element_by_id("catalog-browser")
            .unwrap()
            .remove_attribute("hidden")
            .unwrap();
        let unit_type = doc
            .get_element_by_id("catalog-unit-type")
            .unwrap()
            .unchecked_into::<web_sys::HtmlSelectElement>()
            .value();
        doc.get_element_by_id("catalog-breadcrumbs")
            .unwrap()
            .set_inner_html(&self.breadcrumbs(&unit_type, None));
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = doc
            .get_element_by_id(&format!("catalog-record-{id}"))
            .unwrap()
            .unchecked_ref::<web_sys::HtmlElement>()
            .focus_with_options(&options);
    }

    pub fn update_results(&self, page: usize, animate: bool) {
        use wasm_bindgen::JsCast;
        let doc = crate::document();
        let query = doc
            .get_element_by_id("catalog-query")
            .unwrap()
            .unchecked_into::<web_sys::HtmlInputElement>()
            .value();
        let kind = doc
            .get_element_by_id("catalog-kind")
            .unwrap()
            .unchecked_into::<web_sys::HtmlSelectElement>()
            .value();
        let side = doc
            .get_element_by_id("catalog-side")
            .unwrap()
            .unchecked_into::<web_sys::HtmlSelectElement>()
            .value();
        let unit_type = doc
            .get_element_by_id("catalog-unit-type")
            .unwrap()
            .unchecked_into::<web_sys::HtmlSelectElement>()
            .value();
        let nationality = doc
            .get_element_by_id("catalog-nationality")
            .unwrap()
            .unchecked_into::<web_sys::HtmlSelectElement>()
            .value();
        doc.get_element_by_id("catalog-results")
            .unwrap()
            .set_inner_html(&format!(
                "<div class=\"{}\">{}</div>",
                if animate { "catalog-transition" } else { "" },
                self.results(&query, &kind, &side, &unit_type, &nationality, page)
            ));
        let detail = doc.get_element_by_id("catalog-detail").unwrap();
        detail.set_attribute("hidden", "").unwrap();
        detail.set_inner_html("");
        doc.get_element_by_id("catalog-browser")
            .unwrap()
            .remove_attribute("hidden")
            .unwrap();
        doc.get_element_by_id("catalog-breadcrumbs")
            .unwrap()
            .set_inner_html(&self.breadcrumbs(&unit_type, None));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_displays_the_unit_type_for_a_named_formation() {
        let catalog =
            Catalog::parse(include_str!("../../../public/data/unit-catalog.json")).unwrap();
        let html = catalog.results(
            "oa-116-006-1st-rifle-brigade",
            "formation",
            "allied",
            "motorized-infantry",
            "commonwealth",
            0,
        );
        assert!(html.contains("1st Rifle Brigade"));
        assert!(
            html.contains("Motorized infantry"),
            "source unit type is missing"
        );
        assert!(html.contains("Formation"));
        assert!(html.contains("Commonwealth · Allies"));
        let generic = catalog.results("", "", "", "anti-air-cruiser", "", 0);
        assert!(generic.contains("Anti air cruiser · Unit type</span>"));
        assert!(generic.contains("Showing 1–3 of 3 records"));
        let ship = catalog.results("naval-dido", "naval_unit", "", "anti-air-cruiser", "", 0);
        assert!(ship.contains("Anti air cruiser · Naval unit · Commonwealth · Allies"));
        let index = catalog.results("Dido", "", "", "", "", 0);
        assert!(index.contains("Unit types: 1."));
        assert!(index.contains("Explore Anti air cruiser"));
        assert!(!index.contains("catalog-entry"));
    }

    #[test]
    fn type_index_preserves_empty_types_and_profile_only_members() {
        let mut data: serde_json::Value =
            serde_json::from_str(include_str!("../../../public/data/unit-catalog.json")).unwrap();
        data["entries"] = serde_json::json!([{
            "id": "type-infantry", "name": "Infantry", "kind": "unit_type",
            "nationality": "unspecified", "unit_type": "infantry",
            "verification": "verified", "source_refs": [], "stats": {}
        }]);
        data["profiles"] = serde_json::json!([{
            "id": "profile-recon", "name": "Reference profile", "nationality": "unspecified",
            "unit_type": "recon", "verification": "partial", "source_refs": [], "stats": {}
        }]);
        let catalog = Catalog::parse(&data.to_string()).unwrap();
        let index = catalog.results("", "", "", "", "", 0);
        assert!(index.contains("Explore Infantry"));
        assert!(index.contains("Matching records: 0"));
        assert!(index.contains("Explore Recon"));
        assert!(index.contains("Matching records: 1"));
        let empty = catalog.results("", "", "", "infantry", "", 0);
        assert!(empty.contains("No matching records"));
        assert!(empty.contains("Type definition and sources"));
        let profile = catalog.results("", "profile", "unspecified", "recon", "unspecified", 0);
        assert!(profile.contains("Showing 1–1 of 1 records"));
        assert!(profile.contains("Recon · Characteristic profile · Unspecified · Unspecified"));
        let filtered = catalog.results("", "", "axis", "", "", 0);
        assert!(filtered.contains("No matching unit types"));
    }

    #[test]
    fn catalog_numbers_require_stat_verification_even_when_identity_is_verified() {
        let mut data: serde_json::Value =
            serde_json::from_str(include_str!("../../../public/data/unit-catalog.json")).unwrap();
        Catalog::parse(&data.to_string()).unwrap();
        data["entries"] = serde_json::json!([{
            "id": "test", "name": "Verified identity <not markup>", "kind": "formation", "side": "axis", "nationality": "german", "unit_type": "infantry", "verification": "verified", "source_refs": [],
            "stats": {
                "sourced_zero": {"value": 0, "status": "sourced", "source_refs": [], "note": ""},
                "breakdown_shift": {"value": -1, "raw": "1L", "status": "sourced", "source_refs": [], "note": "Left shift, not a combat rating."},
                "conditional_rating": {"value": 7, "raw": "(7)", "conditional": true, "status": "sourced", "source_refs": [], "note": "Only under the printed chart restriction."},
                "unresolved_number": {"value": 9876, "status": "unresolved", "source_refs": [], "note": ""},
                "provisional_number": {"value": 8765, "status": "provisional", "source_refs": [], "note": ""},
                "printed_dash": {"value": null, "raw": "-", "status": "not_applicable", "source_refs": [], "note": "Weapon mix supplies the rating."},
                "not_applicable": {"value": null, "status": "not_applicable", "source_refs": [], "note": ""}
            }
        }]);
        let catalog = Catalog::parse(&data.to_string()).unwrap();
        let html = catalog.detail("test").unwrap();
        assert!(html.contains("Identity verified"));
        assert!(html.contains("German · Axis"));
        assert!(html.contains("&lt;not markup&gt;"));
        assert!(html.contains("<strong>0</strong>"));
        assert!(html.contains("<strong>1L</strong>"));
        assert!(html.contains("<strong>(7)</strong>"));
        assert!(html.contains("Conditional — see note"));
        assert!(html.contains("Only under the printed chart restriction."));
        assert!(html.contains("Unresolved"));
        assert!(html.contains("Provisional"));
        assert!(html.contains("Source dash"));
        assert!(html.contains("See this chart’s legend and note"));
        assert!(html.contains("Weapon mix supplies the rating."));
        assert!(html.contains("Not applicable"));
        assert!(!html.contains("9876"));
        assert!(!html.contains("8765"));
        assert!(catalog
            .results("GERMAN identity", "formation", "axis", "infantry", "", 0)
            .contains("Identity verified"));
        assert!(catalog
            .results("german identity", "formation", "allied", "infantry", "", 0)
            .contains("No matching records"));
        assert!(catalog
            .results("german identity", "profile", "axis", "infantry", "", 0)
            .contains("No matching records"));
    }

    #[test]
    fn linked_profile_stats_are_attributed_and_do_not_replace_formation_stats() {
        let mut data: serde_json::Value =
            serde_json::from_str(include_str!("../../../public/data/unit-catalog.json")).unwrap();
        data["entries"] = serde_json::json!([{
            "id": "test", "name": "Test formation", "kind": "formation",
            "side": "allied", "nationality": "commonwealth", "unit_type": "infantry",
            "verification": "verified", "source_refs": [], "profile_id": "base",
            "stats": {"capability_points": {"value": null, "status": "unresolved", "note": "Effective CPA unknown", "source_refs": []}}
        }]);
        data["profiles"] = serde_json::json!([{
            "id": "base", "name": "Base <profile>", "unit_type": "infantry",
            "nationality": "commonwealth", "verification": "partial", "source_refs": ["scan-133"],
            "note": "Base only, not deployment state",
            "stats": {
                "capability_points": {"value": 10, "raw": "10+", "status": "sourced", "conditional": true, "note": "Printed restriction", "source_refs": ["scan-133"]},
                "zero": {"value": 0, "status": "sourced", "note": "", "source_refs": []},
                "unknown": {"value": 9876, "status": "unresolved", "note": "", "source_refs": []},
                "pending": {"value": 8765, "status": "provisional", "note": "", "source_refs": []},
                "dash": {"value": null, "status": "not_applicable", "note": "", "source_refs": []}
            }
        }]);
        let catalog = Catalog::parse(&data.to_string()).unwrap();
        let html = catalog.detail("test").unwrap();
        let (direct, linked) = html.split_once("Linked profile characteristics").unwrap();
        assert!(direct.contains("Formation / scenario characteristics"));
        assert!(direct.contains("Effective CPA unknown"));
        assert!(direct.contains("Unresolved"));
        assert!(!direct.contains("<strong>10+</strong>"));
        for text in [
            "Base &lt;profile&gt;",
            "base",
            "Base only, not deployment state",
            "not current effective CPA",
            "<strong>10+</strong>",
            "<strong>0</strong>",
            "Conditional — see note",
            "Printed restriction",
            "PDF p. 133",
            "Unresolved",
            "Provisional",
            "Not applicable",
        ] {
            assert!(linked.contains(text), "Missing {text}");
        }
        assert!(!linked.contains("9876"));
        assert!(!linked.contains("8765"));
    }

    #[test]
    fn aa_formations_show_equipment_references_without_assigning_a_loadout() {
        let catalog =
            Catalog::parse(include_str!("../../../public/data/unit-catalog.json")).unwrap();
        let html = catalog.detail("oa-124-002-13th-light-aa-regt").unwrap();
        assert!(html.contains("Reference weapon ratings — not an assigned loadout"));
        assert!(html.contains("Light AA (Bofors 40mm)"));
        assert!(html.contains("Heavy AA (3.7-inch)"));
        assert!(html.contains("Current TOE and weapon mix are required for a unit total"));
        assert!(!html.contains("20mm M/35 Breda"));
        assert!(!catalog
            .detail("weapons-cw-light-aa-bofors-40mm")
            .unwrap()
            .contains("Reference weapon ratings"));
        assert!(!catalog
            .detail("oa-116-006-1st-rifle-brigade")
            .unwrap()
            .contains("Reference weapon ratings"));
    }

    #[test]
    fn formation_reference_prioritizes_known_base_cpa_and_preserves_arrival_units() {
        let catalog =
            Catalog::parse(include_str!("../../../public/data/unit-catalog.json")).unwrap();
        let machinegun = catalog.detail("oa-149-084-60th-machinegun-bn").unwrap();
        assert!(machinegun.contains("Base capability point allowance"));
        assert!(machinegun.contains("<strong>8</strong><small>CP</small>"));
        assert!(
            machinegun.find("Base capability point allowance").unwrap()
                < machinegun
                    .find("Current / scenario characteristics")
                    .unwrap()
        );
        assert!(!machinegun.contains("not transcribed"));
        assert!(!machinegun.contains("Linked profile characteristics"));
        assert!(machinegun.contains("1/12"));
        assert!(machinegun.contains("<strong>2</strong><small>TOE strength points</small>"));
        let aa = catalog.detail("oa-124-002-13th-light-aa-regt").unwrap();
        assert!(aa.contains("<strong>6</strong><small>Actual AA points</small>"));
        assert!(aa.contains("6 × 1"));
        let armor = catalog.detail("oa-116-003-2nd-bays").unwrap();
        assert!(armor.contains("<strong>28</strong><small>Raw combat points (before ÷10)</small>"));
        let missing = catalog.detail("oa-165-065-gruppe-badde").unwrap();
        assert!(missing.contains("That of the attached units"));
        assert!(missing.contains("No independent quantified loadout"));
    }

    #[test]
    fn catalog_pagination_exposes_every_matching_record() {
        let mut data: serde_json::Value =
            serde_json::from_str(include_str!("../../../public/data/unit-catalog.json")).unwrap();
        data["profiles"] = serde_json::json!([]);
        data["entries"] = (1..=25).map(|n| serde_json::json!({
            "id": format!("regiment-{n}"), "name": format!("Regiment {n:02}"), "kind": "formation",
            "side": "allied", "nationality": "commonwealth", "unit_type": "infantry",
            "verification": "partial", "source_refs": [], "stats": {}
        })).collect();
        let catalog = Catalog::parse(&data.to_string()).unwrap();
        let first = catalog.results(
            "regiment",
            "formation",
            "allied",
            "infantry",
            "commonwealth",
            0,
        );
        let second = catalog.results(
            "regiment",
            "formation",
            "allied",
            "infantry",
            "commonwealth",
            1,
        );
        assert!(first.contains("Showing 1–24 of 25"));
        assert!(first.contains("Regiment 24"));
        assert!(!first.contains("Regiment 25"));
        assert!(second.contains("Showing 25–25 of 25"));
        assert!(second.contains("Regiment 25"));
        assert!(!second.contains("Regiment 24"));
        assert!(catalog
            .results(
                "regiment 25",
                "formation",
                "allied",
                "infantry",
                "commonwealth",
                1
            )
            .contains("Showing 1–1 of 1"));
    }
}
