//! Canonical unit identities and unique artwork, separate from simulation roles.
use crate::view::escape;
use cna_catalog::inventory::Identity;
use cna_sim::{role, Unit, ROLES};

pub struct Inventory(cna_catalog::inventory::Inventory);

impl std::ops::Deref for Inventory {
    type Target = cna_catalog::inventory::Inventory;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Inventory {
    pub fn parse(json: &str) -> Result<Self, String> {
        cna_catalog::inventory::Inventory::parse(json).map(Self)
    }

    fn identity(&self, unit: &Unit) -> Option<&Identity> {
        unit.identity_id.as_deref().and_then(|id| self.find(id))
    }

    pub fn portrait(&self, unit: &Unit) -> String {
        self.identity(unit)
            .filter(|entry| entry.sprite.ready())
            .and_then(|entry| entry.sprite.portrait_path.clone())
            .unwrap_or_else(|| crate::map::sprite(unit))
    }

    pub fn atlas(&self, unit: &Unit) -> String {
        self.identity(unit)
            .filter(|entry| entry.sprite.ready())
            .and_then(|entry| entry.sprite.atlas_path.clone())
            .unwrap_or_else(|| format!("/sprites/directional/{}.png", unit.role().id))
    }

    pub fn artwork_label(&self, unit: &Unit) -> &'static str {
        match self.identity(unit) {
            Some(entry) if entry.sprite.ready() => "Unique identity artwork",
            Some(_) => "Unique art missing — shared role placeholder",
            None => "Prototype role artwork — not a historical identity",
        }
    }

    pub fn panel(&self) -> String {
        let ready = self.entries.iter().filter(|e| e.sprite.ready()).count();
        let roles: String = ROLES.iter().map(|role| format!(
            r#"<article class="deployment-card"><img src="/sprites/{}.png" alt="" loading="lazy"/><h3>{}</h3><p>{}</p><small>{} · {} CP · range {}</small><button data-action="deploy-role" data-role="{}">Place role unit</button></article>"#,
            role.id, role.name, role.description, role.domain_name(), role.cp, role.range, role.id
        )).collect();
        format!(
            r#"<h2>Reinforcements</h2><p>Place a named identity using explicitly fictional sandbox capabilities. Historical source stats are not imported as combat values.</p>
            <p class="inventory-coverage">{ready} of {} candidate identities have ready unique artwork. This is NOT a certified distinct-unit count. Missing art uses a clearly marked shared placeholder; source aliases and unresolved identities remain under review.</p>
            <label class="deployment-side">Command side<select id="deployment-side"><option value="allied">Player controlled</option><option value="axis">Enemy AI</option></select></label>
            <label class="inventory-search">Find an identity<input id="inventory-query" type="search" placeholder="Unit, model, nationality or source ID" data-inventory-filter autofocus/></label>
            <div id="inventory-results">{}</div>
            <details class="sandbox-roles"><summary>Fictional role units ({} shared illustrations, not unique identities)</summary><div class="deployment-grid">{roles}</div></details>"#,
            self.entries.len(),
            self.results("", 0),
            ROLES.len()
        )
    }

    pub fn results(&self, query: &str, page: usize) -> String {
        let words: Vec<_> = query
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let matches: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| {
                let haystack = format!(
                    "{} {} {} {} {}",
                    entry.id,
                    entry.name,
                    entry.side,
                    entry.role_id.as_deref().unwrap_or(""),
                    entry.source_entry_ids.join(" ")
                )
                .to_lowercase();
                words.iter().all(|word| haystack.contains(word))
            })
            .collect();
        if matches.is_empty() {
            return "<p role=\"status\">No matching identities.</p>".into();
        }
        let page = page.min((matches.len() - 1) / 24);
        let start = page * 24;
        let end = (start + 24).min(matches.len());
        let cards: String = matches[start..end].iter().map(|entry| {
            let role = entry.role_id.as_deref().and_then(role);
            let portrait = if entry.sprite.ready() { entry.sprite.portrait_path.clone() }
                else { role.map(|r| format!("/sprites/{}.png", r.id)) };
            let image = portrait.map(|p| format!("<img src=\"{}\" alt=\"\" loading=\"lazy\"/>", escape(&p))).unwrap_or_default();
            format!(r#"<article class="deployment-card">{image}<h3>{}</h3><small>{} · {}</small><p>{}</p><small>{} · {} source rows</small><button data-action="deploy-identity" data-identity="{}" {}>Place identity</button></article>"#,
                escape(&entry.name), escape(&entry.entity_kind), escape(&entry.side),
                if entry.sprite.ready() { "Unique identity artwork" } else { "Unique art missing — shared role placeholder" },
                escape(&entry.identity_status), entry.source_entry_ids.len(), escape(&entry.id),
                if role.is_some() { "" } else { "disabled title=\"No sandbox behavior mapped\"" })
        }).collect();
        format!(
            r#"<p class="inventory-count" role="status" tabindex="-1">Showing {}–{} of {} candidate identities</p><div class="deployment-grid">{cards}</div><nav class="catalog-pages" aria-label="Identity pages"><button data-action="inventory-page" data-page="{}" {}>Previous</button><span>Page {} of {}</span><button data-action="inventory-page" data-page="{}" {}>Next</button></nav>"#,
            start + 1,
            end,
            matches.len(),
            page.saturating_sub(1),
            if page == 0 { "disabled" } else { "" },
            page + 1,
            matches.len().div_ceil(24),
            page + 1,
            if end == matches.len() { "disabled" } else { "" }
        )
    }

    pub fn update_results(&self, page: usize) {
        use wasm_bindgen::JsCast;
        let doc = crate::document();
        let query = doc
            .get_element_by_id("inventory-query")
            .unwrap()
            .unchecked_into::<web_sys::HtmlInputElement>()
            .value();
        doc.get_element_by_id("inventory-results")
            .unwrap()
            .set_inner_html(&self.results(&query, page));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_sharing_a_role_keep_distinct_art_and_missing_art_stays_explicit() {
        let entries: Vec<_> = ["first", "second", "unreviewed"].iter().map(|id| serde_json::json!({
            "id": id, "name": format!("Unit {id}"), "role_id": "crusader", "side": "allied",
            "entity_kind": "formation_candidate", "identity_status": "provisional", "source_entry_ids": [format!("source-{id}")],
            "sprite": {"status": if *id == "unreviewed" { "candidate" } else { "ready" },
                "portrait_path": format!("/sprites/unique/{id}.png"), "atlas_path": format!("/sprites/unique/{id}-atlas.png")}
        })).collect();
        let inventory = Inventory::parse(
            &serde_json::json!({"schema_version":1,"entries":entries}).to_string(),
        )
        .unwrap();
        let mut unit = cna_sim::new_game().units.remove(0);
        for id in ["first", "second"] {
            unit.identity_id = Some(id.into());
            assert_eq!(
                inventory.portrait(&unit),
                format!("/sprites/unique/{id}.png")
            );
            assert_eq!(
                inventory.atlas(&unit),
                format!("/sprites/unique/{id}-atlas.png")
            );
        }
        unit.identity_id = Some("unreviewed".into());
        assert_eq!(inventory.portrait(&unit), "/sprites/crusader.png");
        assert_eq!(inventory.atlas(&unit), "/sprites/directional/crusader.png");
        assert!(inventory.artwork_label(&unit).contains("missing"));
        assert!(inventory.panel().contains("2 of 3 candidate identities"));
        assert!(inventory
            .results("source-second", 0)
            .contains("Unit second"));
        assert!(!inventory.results("source-second", 0).contains("Unit first"));
        let bundled =
            Inventory::parse(include_str!("../../../public/data/unit-identities.json")).unwrap();
        assert!(bundled
            .entries
            .iter()
            .all(|entry| entry.role_id.as_deref().is_none_or(|id| role(id).is_some())));
    }
}
