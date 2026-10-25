use cna_catalog::{catalog::Catalog, inventory::Inventory};
use serde_json::json;

const IDENTITIES: &str = include_str!("../../../public/data/unit-identities.json");

#[test]
fn bundled_sources_parse_and_all_retired_ids_resolve() {
    let catalog = Catalog::parse(include_str!("../../../public/data/unit-catalog.json")).unwrap();
    assert_eq!(catalog.entries.len(), 1450);
    assert_eq!(catalog.profiles.len(), 124);
    assert!(catalog.profiles.iter().all(|p| p.kind == "profile"));
    let inventory = Inventory::parse(IDENTITIES).unwrap();
    assert_eq!(inventory.entries.len(), 1356);
    let source: serde_json::Value = serde_json::from_str(IDENTITIES).unwrap();
    let redirects = source["identity_redirects"].as_object().unwrap();
    assert_eq!(redirects.len(), 18);
    for (retired, target) in redirects {
        assert_eq!(
            inventory.find(retired).map(|entry| entry.id.as_str()),
            target.as_str()
        );
    }
}

#[test]
fn redirects_follow_chains_and_reject_ambiguous_or_broken_graphs() {
    let source: serde_json::Value = serde_json::from_str(IDENTITIES).unwrap();
    let target = source["entries"][0]["id"].as_str().unwrap();
    let other = source["entries"][1]["id"].as_str().unwrap();
    let mut fixture = json!({
        "schema_version": 1,
        "entries": [source["entries"][0].clone(), source["entries"][1].clone()],
        "identity_redirects": {"old": "middle", "middle": target}
    });
    let inventory = Inventory::parse(&fixture.to_string()).unwrap();
    assert_eq!(inventory.find("old").unwrap().id, target);
    assert!(inventory.find("unknown").is_none());

    fixture["identity_redirects"] = json!({"old":"middle", "middle":"old"});
    assert!(
        Inventory::parse(&fixture.to_string()).is_err(),
        "loop accepted"
    );
    fixture["identity_redirects"] = json!({"old":"unknown"});
    assert!(
        Inventory::parse(&fixture.to_string()).is_err(),
        "dangling target accepted"
    );
    fixture["identity_redirects"] = json!({target: other});
    assert!(
        Inventory::parse(&fixture.to_string()).is_err(),
        "live ID also accepted as retired"
    );
    fixture["identity_redirects"] = json!({});
    let duplicate_keys = fixture.to_string().replace(
        "\"identity_redirects\":{}",
        &format!(r#""identity_redirects":{{"old":"{target}","old":"{other}"}}"#),
    );
    assert!(
        Inventory::parse(&duplicate_keys).is_err(),
        "duplicate redirect key accepted"
    );
}

#[test]
fn source_art_requires_reviewed_complete_assets_and_matches_source_rows() {
    let mut fixture = json!({
        "schema_version": 1,
        "entries": [{
            "id": "canonical-unit", "name": "Unit", "role_id": null,
            "side": "allied", "entity_kind": "formation_candidate",
            "identity_status": "source_identified",
            "source_entry_ids": ["oa-row", "counter-row"],
            "sprite": {"status": "ready", "portrait_path": "/portrait.png", "atlas_path": "/atlas.png"}
        }]
    });
    for (status, portrait, atlas, expected) in [
        (
            "ready",
            Some("/portrait.png"),
            Some("/atlas.png"),
            Some("/portrait.png"),
        ),
        ("candidate", Some("/portrait.png"), Some("/atlas.png"), None),
        ("ready", Some("/portrait.png"), None, None),
        ("ready", None, Some("/atlas.png"), None),
        ("missing", None, None, None),
    ] {
        fixture["entries"][0]["sprite"] = json!({
            "status": status, "portrait_path": portrait, "atlas_path": atlas
        });
        let inventory = Inventory::parse(&fixture.to_string()).unwrap();
        let note = if expected.is_some() {
            "Unique identity artwork"
        } else {
            "Unique art missing — shared role placeholder"
        };
        for source in ["oa-row", "counter-row"] {
            assert_eq!(inventory.source_art(source), Some((expected, note)));
        }
        assert_eq!(inventory.source_art("canonical-unit"), None);
        assert_eq!(inventory.source_art("unknown-row"), None);
    }
}

#[test]
fn instance_references_reject_stats_instead_of_silently_discarding_them() {
    use cna_catalog::ScenarioInstanceRef;
    let reference: ScenarioInstanceRef =
        serde_json::from_value(json!({"id":"scenario-7"})).unwrap();
    assert_eq!(reference.id, "scenario-7");
    assert!(serde_json::from_value::<ScenarioInstanceRef>(json!({
        "id":"scenario-7", "stats":{"strength":5}
    }))
    .is_err());
}
