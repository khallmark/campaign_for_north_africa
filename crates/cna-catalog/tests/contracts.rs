use cna_catalog::{catalog::Catalog, inventory::Inventory};
use serde_json::json;

const IDENTITIES: &str = include_str!("../../../public/data/unit-identities.json");

#[test]
fn graziani_fixture_keeps_sample_identity_attachments_and_unknown_strength() {
    use cna_catalog::{fixture, IdentityStatus, ValueStatus, CATALOG_CONTRACT_VERSION};
    use std::collections::BTreeSet;

    let sample = fixture::graziani().unwrap();
    assert_eq!(CATALOG_CONTRACT_VERSION, 1);
    assert_eq!(sample.contract_version, CATALOG_CONTRACT_VERSION);
    assert_eq!(sample.status, IdentityStatus::Fixture);
    assert_eq!(sample.units.len(), 17);
    assert_eq!(
        sample
            .units
            .iter()
            .filter(|r| r.unit.side == "axis")
            .count(),
        9
    );
    assert_eq!(
        sample
            .units
            .iter()
            .filter(|r| r.unit.side == "allied")
            .count(),
        8
    );
    let ids: BTreeSet<_> = sample.units.iter().map(|r| &r.unit.id).collect();
    assert_eq!(ids.len(), sample.units.len());
    for record in &sample.units {
        assert!(record.unit.id.0.starts_with("fixture-graziani-"));
        assert_eq!(record.unit.status, IdentityStatus::Fixture);
        assert!(!record.unit.source_refs.is_empty());
        assert_eq!(record.toe.unit_id, record.unit.id);
        assert_eq!(record.toe.personnel.status, ValueStatus::Unverified);
        assert_eq!(record.toe.personnel.value, None);
        assert!(
            record.toe.equipment.is_empty(),
            "no fabricated equipment counts"
        );
        if let Some(parent) = &record.attached_to {
            assert!(ids.contains(parent));
            assert_ne!(parent, &record.unit.id);
        }
    }
    let attachments: BTreeSet<_> = sample
        .units
        .iter()
        .filter_map(|r| {
            r.attached_to
                .as_ref()
                .map(|parent| (r.unit.id.0.as_str(), parent.0.as_str()))
        })
        .collect();
    assert_eq!(
        attachments,
        BTreeSet::from([
            ("fixture-graziani-i-m", "fixture-graziani-1-ccnn"),
            ("fixture-graziani-ix-l", "fixture-graziani-2-libyan"),
            ("fixture-graziani-ii-m", "fixture-graziani-maletti"),
            ("fixture-graziani-1-essex", "fixture-graziani-matruh"),
        ])
    );
    for (id, name) in [
        ("fixture-graziani-7-armored", "7th Armored Division"),
        ("fixture-graziani-4-indian", "4th Indian Division"),
    ] {
        let unit = &sample
            .units
            .iter()
            .find(|r| r.unit.id.0 == id)
            .unwrap()
            .unit;
        assert_eq!(unit.names, [name]);
        assert_eq!(unit.kind, "division");
    }
    let raw: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/graziani-sample.json")).unwrap();
    assert_eq!(serde_json::to_value(sample).unwrap(), raw);
}

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

#[test]
fn schema_examples_match_reference_types() {
    use cna_catalog::*;
    let docs = include_str!("../../../docs/contracts/catalog-v1.md");
    let examples: serde_json::Value = serde_json::from_str(
        docs.split_once("```json\n")
            .unwrap()
            .1
            .split_once("\n```")
            .unwrap()
            .0,
    )
    .unwrap();

    fn check<T: serde::de::DeserializeOwned + serde::Serialize>(
        examples: &serde_json::Value,
        name: &str,
    ) {
        let value = examples[name].clone();
        let typed: T = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), value, "{name}");
        if let Some(mut object) = value.as_object().cloned() {
            object.insert("live_strength".into(), json!(7));
            assert!(
                serde_json::from_value::<T>(json!(object)).is_err(),
                "{name}"
            );
        }
    }

    check::<CanonicalUnitId>(&examples, "CanonicalUnitId");
    check::<IdentityStatus>(&examples, "IdentityStatus");
    check::<AssetStatus>(&examples, "AssetStatus");
    check::<ValueStatus>(&examples, "ValueStatus");
    check::<CanonicalUnit>(&examples, "CanonicalUnit");
    check::<CounterRepresentation>(&examples, "CounterRepresentation");
    check::<StateVersion>(&examples, "StateVersion");
    check::<ScenarioInstanceRef>(&examples, "ScenarioInstanceRef");
    check::<AssetRef>(&examples, "AssetRef");
    check::<ReferenceValue<String>>(&examples, "ReferenceValue");
    check::<ReferenceValue<u32>>(&examples, "CountReference");
    check::<ReferenceValue<f64>>(&examples, "NumberReference");
    check::<Toe>(&examples, "Toe");
    check::<Equipment>(&examples, "Equipment");
}
