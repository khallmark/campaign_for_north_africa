use cna_sim::official::state::*;
use std::collections::{BTreeMap, BTreeSet};

fn fixture() -> (GameState, BTreeSet<cna_catalog::CanonicalUnitId>) {
    let unit_id = cna_catalog::CanonicalUnitId("fixture-unit".into());
    let unit = UnitInstance {
        unit_id: unit_id.clone(),
        side: Side::Commonwealth,
        location: "A3033".parse().unwrap(),
        counter_states: BTreeMap::from([(
            "fixture-axis".into(),
            cna_catalog::StateVersion {
                axis: "fixture-axis".into(),
                level: "fixture-level".into(),
                source: "fixture-source".into(),
            },
        )]),
        toe: BTreeMap::from([("fixture-infantry".into(), ToePoints::whole(7))]),
        supplies: Supplies {
            fuel: FuelPoints::new(7, 2).unwrap(),
            ammunition: AmmunitionPoints::whole(3),
            stores: StoresPoints::whole(11),
            water: WaterPoints::whole(5),
        },
        trucks: BTreeMap::from([(
            TruckType::Light,
            BTreeMap::from([(TruckLine::First, TruckPoints::whole(2))]),
        )]),
    };
    let state = GameState {
        revision: 0,
        calendar: Calendar::new(1, 1).unwrap(),
        mode: Mode::DetailedLogisticsAbstractAir58,
        initiative: None,
        instances: BTreeMap::from([
            ("fixture-a".into(), unit.clone()),
            ("fixture-b".into(), unit),
        ]),
        organizations: BTreeMap::from([(
            "fixture-a".into(),
            Organization {
                assigned_to: Some("fixture-b".into()),
                attached_to: None,
            },
        )]),
        cargo_batches: BTreeMap::from([(
            "fixture-batch".into(),
            CargoBatch {
                side: Side::Commonwealth,
                holder: CargoHolder::Instance("fixture-a".into()),
                supplies: Supplies::default(),
            },
        )]),
        pending_commitments: BTreeMap::from([(
            "fixture-commitment".into(),
            Commitment {
                side: Side::Commonwealth,
                instances: BTreeSet::from(["fixture-b".into()]),
                cargo_batches: BTreeSet::from(["fixture-batch".into()]),
            },
        )]),
        rng: RngState {
            seed: [7; 32],
            word_position: 19,
        },
        references: ReferenceDigests {
            ruleset_id: "fixture-only".into(),
            ruleset: format!("sha256:{}", "1".repeat(64)),
            map: format!("sha256:{}", "2".repeat(64)),
            catalog: format!("sha256:{}", "3".repeat(64)),
        },
    };
    (state, BTreeSet::from([unit_id]))
}

#[test]
fn official_state_round_trips_exact_quantities_and_stable_identity_order() {
    let (state, catalog) = fixture();
    state.validate(&catalog).unwrap();
    let bytes = serde_json::to_vec(&state).unwrap();
    let restored: GameState = serde_json::from_slice(&bytes).unwrap();
    restored.validate(&catalog).unwrap();
    assert_eq!(restored, state);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    let mut reversed = state.clone();
    reversed.instances = state.instances.into_iter().rev().collect();
    assert_eq!(serde_json::to_vec(&reversed).unwrap(), bytes);
    assert_eq!(
        restored.instances["fixture-a"].supplies.fuel,
        FuelPoints::new(7, 2).unwrap()
    );
    assert_eq!(restored.rng.word_position, 19);
    assert_eq!(
        restored.instances["fixture-a"].trucks[&TruckType::Light][&TruckLine::First],
        TruckPoints::whole(2)
    );
    let mut invalid = serde_json::to_value(&restored).unwrap();
    invalid["instances"]["fixture-a"]["hp"] = 100.into();
    assert!(serde_json::from_value::<GameState>(invalid).is_err());
    assert!(FuelPoints::new(1, 0).is_err());
    assert!(serde_json::from_str::<FuelPoints>(r#"{"numerator":1,"denominator":0}"#).is_err());
}

#[test]
fn official_state_rejects_dangling_references_in_every_relationship() {
    let (state, catalog) = fixture();
    let mut invalid = state.clone();
    let organization = invalid.organizations.remove("fixture-a").unwrap();
    invalid.organizations.insert("missing".into(), organization);
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = state.clone();
    invalid
        .organizations
        .get_mut("fixture-a")
        .unwrap()
        .assigned_to = Some("missing".into());
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = state.clone();
    invalid
        .organizations
        .get_mut("fixture-a")
        .unwrap()
        .attached_to = Some("missing".into());
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = state.clone();
    invalid
        .cargo_batches
        .get_mut("fixture-batch")
        .unwrap()
        .holder = CargoHolder::Instance("missing".into());
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = state.clone();
    invalid
        .pending_commitments
        .get_mut("fixture-commitment")
        .unwrap()
        .instances
        .insert("missing".into());
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = state.clone();
    invalid.cargo_batches.clear();
    assert!(invalid.validate(&catalog).is_err());
    assert!(state.validate(&BTreeSet::new()).is_err());
    let mut invalid = state.clone();
    invalid
        .instances
        .get_mut("fixture-a")
        .unwrap()
        .counter_states
        .get_mut("fixture-axis")
        .unwrap()
        .axis = "different-axis".into();
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = state;
    invalid.references.ruleset = "unreviewed".into();
    assert!(invalid.validate(&catalog).is_err());
}

#[test]
fn official_calendar_advances_three_stages_per_week_without_wrapping_campaign() {
    for (turn, year, month, week) in [
        (1, 1940, 9, 3),
        (3, 1940, 10, 1),
        (15, 1941, 1, 1),
        (111, 1943, 1, 1),
    ] {
        let calendar = Calendar::new(turn, 1).unwrap();
        assert_eq!(calendar.date(), CampaignDate { year, month, week });
    }
    let mut cursor = Calendar::new(1, 1).unwrap();
    for step in 1..333 {
        cursor = cursor.next_operations_stage().unwrap();
        assert_eq!(cursor.game_turn(), step / 3 + 1);
        assert_eq!(u16::from(cursor.operations_stage()), step % 3 + 1);
    }
    assert!(cursor.next_operations_stage().is_none());
    assert!(Calendar::new(0, 1).is_err());
    assert!(Calendar::new(112, 1).is_err());
    assert!(Calendar::new(1, 0).is_err());
    assert!(Calendar::new(1, 4).is_err());
    assert!(serde_json::from_str::<Calendar>(r#"{"game_turn":1,"operations_stage":4}"#).is_err());
}
