use crate::view::escape;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

const PAGE_SIZE: usize = 24;

#[derive(Deserialize)]
pub struct Catalog {
    schema_version: u32,
    coverage: Coverage,
    entries: Vec<Record>,
    profiles: Vec<Record>,
    stat_definitions: Vec<StatDefinition>,
    sources: Vec<Source>,
    unit_types: Vec<UnitType>,
    sprites: Vec<Sprite>,
}

#[derive(Deserialize)]
struct Coverage {
    complete_historical_roster: bool,
    summary: String,
    gaps: Vec<String>,
}

#[derive(Deserialize)]
struct Record {
    id: String,
    name: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    side: String,
    nationality: String,
    unit_type: String,
    profile_id: Option<String>,
    verification: String,
    source_refs: Vec<String>,
    stats: BTreeMap<String, Stat>,
    sprite: Option<Sprite>,
    #[serde(skip)]
    artwork_note: String,
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
struct Stat {
    value: Option<f64>,
    status: String,
    source_refs: Vec<String>,
    note: String,
    raw: Option<String>,
    #[serde(default)]
    conditional: bool,
}

#[derive(Deserialize)]
struct StatDefinition {
    id: String,
    name: String,
    unit: String,
    description: String,
}

#[derive(Deserialize)]
struct Source {
    id: String,
    title: String,
    path: String,
    section: Option<String>,
    pdf_page: Option<u32>,
    note: String,
}

#[derive(Clone, Deserialize)]
struct Sprite {
    #[serde(default)]
    id: String,
    path: String,
}

#[derive(Deserialize)]
struct UnitType {
    id: String,
    sprite_id: String,
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

impl Catalog {
    pub fn apply_identity_artwork(&mut self, inventory: &crate::inventory::Inventory) {
        for record in self.entries.iter_mut().chain(&mut self.profiles) {
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
        let mut catalog: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if catalog.schema_version != 1 {
            return Err("Unsupported catalog schema".into());
        }
        // Profiles are reference records, not formations or deployable scenario units.
        for profile in &mut catalog.profiles {
            profile.kind = "profile".into();
            if profile.sprite.is_none() {
                profile.sprite = catalog
                    .unit_types
                    .iter()
                    .find(|role| role.id == profile.unit_type)
                    .and_then(|role| {
                        catalog
                            .sprites
                            .iter()
                            .find(|sprite| sprite.id == role.sprite_id)
                    })
                    .cloned();
            }
        }
        Ok(catalog)
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
        format!(
            r#"<h2>Unit catalog</h2>
            <p>Browse rulebook records and illustrative sprites. These are references, not deployable units; the battlefield still uses simplified tutorial stats.</p>
            <details class="catalog-coverage"><summary>{}</summary><p>{}</p><ul>{gaps}</ul><p>Identity verification does not verify every stat. Only sourced numbers are displayed. PDF citations use one-based physical pages.</p></details>
            <div class="catalog-filters"><label>Search records<input id="catalog-query" type="search" placeholder="Name, nationality, role or ID" data-catalog-filter autofocus /></label>
            <label>Record type<select id="catalog-kind" data-catalog-filter><option value="">All record types</option>{kinds}</select></label>
            <label>Side<select id="catalog-side" data-catalog-filter><option value="">All sides</option><option value="allied">Allied</option><option value="axis">Axis</option><option value="unspecified">Unspecified</option></select></label></div>
            <div id="catalog-results">{}</div>"#,
            if self.coverage.complete_historical_roster {
                "Coverage and source limitations"
            } else {
                "Roster transcription incomplete — coverage and limitations"
            },
            escape(&self.coverage.summary),
            self.results("", "", "", 0)
        )
    }

    pub fn results(&self, query: &str, kind: &str, side: &str, page: usize) -> String {
        let query = query.to_lowercase();
        let matches: Vec<_> = self
            .entries
            .iter()
            .chain(&self.profiles)
            .filter(|record| {
                let record_side = if record.side.is_empty() {
                    "unspecified"
                } else {
                    &record.side
                };
                let text = format!(
                    "{} {} {} {} {}",
                    record.name,
                    record.id,
                    record.nationality,
                    record.unit_type,
                    kind_name(&record.kind)
                )
                .to_lowercase();
                (kind.is_empty() || record.kind == kind)
                    && (side.is_empty() || record_side == side)
                    && query.split_whitespace().all(|word| text.contains(word))
            })
            .collect();
        if matches.is_empty() {
            return "<p class=\"catalog-count\" role=\"status\" tabindex=\"-1\">No matching records. Try a shorter search or choose all record types and sides.</p>".into();
        }
        let page = page.min((matches.len() - 1) / PAGE_SIZE);
        let start = page * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(matches.len());
        let records: String = matches[start..end]
            .iter()
            .map(|record| self.record(record))
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
            r#"<p class="catalog-count" role="status" tabindex="-1">Showing {}–{} of {} records. Expand a record for stats and sources.</p><div class="catalog-list">{records}</div>{pagination}"#,
            start + 1,
            end,
            matches.len()
        )
    }

    fn record(&self, record: &Record) -> String {
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
        let mut stats: Vec<_> = record.stats.iter().collect();
        stats.sort_by_key(|(id, _)| {
            self.stat_definitions
                .iter()
                .position(|definition| definition.id == id.as_str())
                .unwrap_or(usize::MAX)
        });
        let stats: String = stats.into_iter().map(|(id, stat)| {
            let definition = self.stat_definitions.iter().find(|d| d.id == *id);
            let name = definition.map(|d| d.name.as_str()).unwrap_or(id);
            let description = definition.map(|d| d.description.as_str()).unwrap_or("");
            let value = match (stat.status.as_str(), stat.value) {
                ("sourced", Some(value)) => {
                    let printed = stat.raw.clone().unwrap_or_else(|| value.to_string());
                    format!("<strong>{}</strong><small>{}</small>{}", escape(&printed), escape(definition.map(|d| d.unit.as_str()).unwrap_or("")), if stat.conditional { "<small><b>Conditional — see note</b></small>" } else { "" })
                },
                ("not_applicable", _) => "Not applicable".into(),
                ("provisional", _) => "Provisional<br><small>Value withheld</small>".into(),
                _ => "Unresolved".into(),
            };
            format!("<tr><th scope=\"row\">{}<small>{}</small><small>{}</small><small>{}</small></th><td>{value}</td></tr>", escape(name), escape(description), escape(&stat.note), self.citations(&stat.source_refs))
        }).collect();
        let stats = if stats.is_empty() {
            "<p>No numerical characteristics have been recorded for this entry.</p>".into()
        } else {
            format!("<table><caption>Rulebook characteristics, not tutorial stats</caption><thead><tr><th scope=\"col\">Characteristic and source</th><th scope=\"col\">Value</th></tr></thead><tbody>{stats}</tbody></table>")
        };
        let profile = record.profile_id.as_ref().map(|id| format!("<p>Characteristic profile: {}. Search this ID to inspect the profile separately.</p>", escape(id))).unwrap_or_default();
        format!(
            r#"<details class="catalog-entry"><summary>{sprite}<span class="catalog-name"><strong>{}</strong><span>{} · {} · {}</span><small>{}</small></span><span class="catalog-status">{identity}</span></summary><div class="catalog-record"><p class="catalog-id">Record {}</p><p>{}</p>{profile}{stats}<details class="catalog-citations"><summary>Record sources</summary>{}</details></div></details>"#,
            escape(&record.name),
            escape(kind_name(&record.kind)),
            escape(&record.nationality.replace('_', " ")),
            escape(if record.side.is_empty() {
                "unspecified"
            } else {
                &record.side
            }),
            escape(&record.artwork_note),
            escape(&record.id),
            escape(&record.note),
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

    pub fn update_results(&self, page: usize) {
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
        doc.get_element_by_id("catalog-results")
            .unwrap()
            .set_inner_html(&self.results(&query, &kind, &side, page));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_numbers_require_stat_verification_even_when_identity_is_verified() {
        let mut data: serde_json::Value =
            serde_json::from_str(include_str!("../../../public/data/unit-catalog.json")).unwrap();
        Catalog::parse(&data.to_string()).unwrap();
        data["entries"] = serde_json::json!([{
            "id": "test", "name": "Verified identity <not markup>", "kind": "unit_type", "side": "axis", "nationality": "german", "unit_type": "infantry", "verification": "verified", "source_refs": [],
            "stats": {
                "sourced_zero": {"value": 0, "status": "sourced", "source_refs": [], "note": ""},
                "breakdown_shift": {"value": -1, "raw": "1L", "status": "sourced", "source_refs": [], "note": "Left shift, not a combat rating."},
                "conditional_rating": {"value": 7, "raw": "(7)", "conditional": true, "status": "sourced", "source_refs": [], "note": "Only under the printed chart restriction."},
                "unresolved_number": {"value": 9876, "status": "unresolved", "source_refs": [], "note": ""},
                "provisional_number": {"value": 8765, "status": "provisional", "source_refs": [], "note": ""},
                "not_applicable": {"value": null, "status": "not_applicable", "source_refs": [], "note": ""}
            }
        }]);
        let catalog = Catalog::parse(&data.to_string()).unwrap();
        let html = catalog.results("verified identity", "unit_type", "axis", 0);
        assert!(html.contains("Identity verified"));
        assert!(html.contains("&lt;not markup&gt;"));
        assert!(html.contains("<strong>0</strong>"));
        assert!(html.contains("<strong>1L</strong>"));
        assert!(html.contains("<strong>(7)</strong>"));
        assert!(html.contains("Conditional — see note"));
        assert!(html.contains("Only under the printed chart restriction."));
        assert!(html.contains("Unresolved"));
        assert!(html.contains("Provisional"));
        assert!(html.contains("Not applicable"));
        assert!(!html.contains("9876"));
        assert!(!html.contains("8765"));
        assert!(catalog
            .results("GERMAN identity", "unit_type", "axis", 0)
            .contains("Identity verified"));
        assert!(catalog
            .results("german identity", "unit_type", "allied", 0)
            .contains("No matching records"));
        assert!(catalog
            .results("german identity", "profile", "axis", 0)
            .contains("No matching records"));
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
        let first = catalog.results("regiment", "formation", "allied", 0);
        let second = catalog.results("regiment", "formation", "allied", 1);
        assert!(first.contains("Showing 1–24 of 25"));
        assert!(first.contains("Regiment 24"));
        assert!(!first.contains("Regiment 25"));
        assert!(second.contains("Showing 25–25 of 25"));
        assert!(second.contains("Regiment 25"));
        assert!(!second.contains("Regiment 24"));
        assert!(catalog
            .results("regiment 25", "formation", "allied", 1)
            .contains("Showing 1–1 of 1"));
    }
}
