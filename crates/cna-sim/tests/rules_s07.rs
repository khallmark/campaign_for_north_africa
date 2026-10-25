//! L21 §7, physical p14; R79 physical pp13/97 (rules/chart).
//! Synthetic positions test printed rules, not historical deployments.
use cna_catalog::CanonicalUnitId;
use cna_sim::{
    contract::command::{Command, Outcome, SchemaVersion},
    official::{
        initiative::{
            InitiativeCommand, InitiativeEvent, InitiativeRole, InitiativeRules, OpeningInitiative,
        },
        reducer::{Session, Unavailable},
        state::*,
    },
};
use std::collections::{BTreeMap, BTreeSet};

fn position(turn: u16) -> GameState {
    GameState {
        revision: 0,
        calendar: Calendar::new(turn, 1).unwrap(),
        mode: Mode::DetailedLogisticsAbstractAir58,
        initiative: None,
        instances: BTreeMap::new(),
        organizations: BTreeMap::new(),
        cargo_batches: BTreeMap::new(),
        pending_commitments: BTreeMap::new(),
        rng: RngState {
            seed: [0; 32],
            word_position: 0,
        },
        references: ReferenceDigests {
            ruleset_id: "initiative-fixture".into(),
            ruleset: format!("sha256:{}", "1".repeat(64)),
            map: format!("sha256:{}", "2".repeat(64)),
            catalog: format!("sha256:{}", "3".repeat(64)),
        },
    }
}

fn add(game: &mut GameState, id: &str, location: &str) {
    game.instances.insert(
        id.into(),
        UnitInstance {
            unit_id: CanonicalUnitId(id.into()),
            side: Side::Axis,
            location: location.parse().unwrap(),
            counter_states: BTreeMap::new(),
            toe: BTreeMap::new(),
            supplies: Supplies::default(),
            trucks: BTreeMap::new(),
        },
    );
}

fn session(game: GameState) -> Session<InitiativeCommand, InitiativeEvent> {
    let ids: BTreeSet<_> = game.instances.values().map(|u| u.unit_id.clone()).collect();
    Session::new(
        game,
        BTreeMap::from([
            ("axis".into(), Side::Axis),
            ("cw".into(), Side::Commonwealth),
        ]),
        ids,
    )
    .unwrap()
}

fn command(
    id: &str,
    revision: u64,
    actor: &str,
    payload: InitiativeCommand,
) -> Command<InitiativeCommand> {
    Command {
        schema_version: SchemaVersion::V1,
        ruleset_id: "initiative-fixture".into(),
        ruleset_digest: format!("sha256:{}", "1".repeat(64)),
        actor: actor.into(),
        game_revision: revision,
        command_id: id.into(),
        payload,
    }
}

fn execute(
    session: &mut Session<InitiativeCommand, InitiativeEvent>,
    rules: &InitiativeRules,
    cmd: &Command<InitiativeCommand>,
) -> Outcome<InitiativeEvent> {
    session.reduce(&cmd.actor, cmd, |side, order, tx| {
        rules.execute(side, order, tx)
    })
}

#[test]
fn original_chart_boundaries_and_rommel_presence_determine_ratings() {
    // R79 p97 §7.2, independently inspected against the VASSAL chart image.
    let rules = InitiativeRules {
        roles: BTreeMap::from([
            (CanonicalUnitId("rommel".into()), InitiativeRole::Rommel),
            (
                CanonicalUnitId("german".into()),
                InitiativeRole::GermanLandCombat,
            ),
            (CanonicalUnitId("italian".into()), InitiativeRole::Other),
        ]),
        opening: None,
    };
    for (turn, expected) in [(1, 3), (42, 3), (43, 4), (90, 4), (91, 5), (111, 5)] {
        let mut game = position(turn);
        add(&mut game, "italian", "A3033");
        assert_eq!(rules.ratings(&game).unwrap().commonwealth, expected);
        assert_eq!(rules.ratings(&game).unwrap().axis, 1);
        add(&mut game, "german", "A3034");
        assert_eq!(rules.ratings(&game).unwrap().axis, 3);
        add(&mut game, "rommel", "A3035");
        assert_eq!(rules.ratings(&game).unwrap().axis, 6);
        for holding in ["Tripoli", "Tripolitania", "Gabes", "Tunis"] {
            game.instances.get_mut("rommel").unwrap().location = holding.parse().unwrap();
            assert_eq!(rules.ratings(&game).unwrap().axis, 3);
            game.instances.get_mut("german").unwrap().location = holding.parse().unwrap();
            assert_eq!(rules.ratings(&game).unwrap().axis, 1);
            game.instances.get_mut("german").unwrap().location = "A3034".parse().unwrap();
        }
    }
    // L21 p14 §7.14's printed October 1940 example: 1+4 beats 3+1.
    let ratings = rules.ratings(&position(3)).unwrap();
    assert_eq!(ratings.winner(4, 1).unwrap(), Some(Side::Axis));
    assert_eq!(ratings.winner(3, 1).unwrap(), None);
    assert!(ratings.winner(0, 1).is_err());
    assert!(ratings.winner(6, 7).is_err());
}

#[test]
fn ties_reroll_both_dice_and_exact_retries_do_not_reroll_the_week() {
    let rules = InitiativeRules {
        roles: BTreeMap::from([(CanonicalUnitId("rommel".into()), InitiativeRole::Rommel)]),
        opening: None,
    };
    let mut initial = position(1);
    add(&mut initial, "rommel", "A3033");
    let mut game = session(initial.clone());
    let cmd = command("roll", 0, "axis", InitiativeCommand::Determine {});
    let accepted = execute(&mut game, &rules, &cmd);
    assert!(matches!(
        accepted,
        Outcome::Accepted {
            new_revision: 1,
            ..
        }
    ));
    // Independent ChaCha8 TC1 words: [1,4] => 7:7; [2,4] => 8:7.
    assert_eq!(
        game.events()
            .iter()
            .map(|e| e.payload.clone())
            .collect::<Vec<_>>(),
        vec![
            InitiativeEvent::Rolled {
                game_turn: 1,
                axis_rating: 6,
                commonwealth_rating: 3,
                axis_die: 1,
                commonwealth_die: 4,
                winner: None
            },
            InitiativeEvent::Rolled {
                game_turn: 1,
                axis_rating: 6,
                commonwealth_rating: 3,
                axis_die: 2,
                commonwealth_die: 4,
                winner: Some(Side::Axis)
            },
        ]
    );
    assert_eq!(
        game.events()
            .iter()
            .flat_map(|e| &e.random_draws)
            .map(|d| d.value)
            .collect::<Vec<_>>(),
        [0x2fef003e, 0xd6405f89, 0xe8b85b7f, 0xa1a5091f]
    );
    assert_eq!(game.state().rng.word_position, 4);
    assert_eq!(game.state().initiative.as_ref().unwrap().winner, Side::Axis);
    let before = serde_json::to_vec(&game).unwrap();
    assert_eq!(execute(&mut game, &rules, &cmd), accepted);
    assert_eq!(
        execute(
            &mut game,
            &rules,
            &command("reroll", 1, "cw", InitiativeCommand::Determine {})
        ),
        Unavailable.outcome()
    );
    assert_eq!(serde_json::to_vec(&game).unwrap(), before);
    let mut again = session(initial);
    assert_eq!(execute(&mut again, &rules, &cmd), accepted);
    assert_eq!(serde_json::to_vec(&again).unwrap(), before);
}

#[test]
fn scenario_override_lasts_one_week_and_winner_chooses_once_per_stage() {
    // SC21 p5 §60.6: Italy gets GT1; normal determination resumes GT2.
    let rules = InitiativeRules {
        roles: BTreeMap::new(),
        opening: Some(OpeningInitiative {
            at: Calendar::new(1, 1).unwrap(),
            winner: Side::Axis,
        }),
    };
    let mut game = session(position(1));
    execute(
        &mut game,
        &rules,
        &command("opening", 0, "cw", InitiativeCommand::Determine {}),
    );
    assert_eq!(game.state().rng.word_position, 0);
    assert_eq!(
        game.events()[0].payload,
        InitiativeEvent::ScenarioInitiative {
            at: Calendar::new(1, 1).unwrap(),
            winner: Side::Axis,
        }
    );
    for (stage, first) in [(1, true), (2, false), (3, true)] {
        // Trusted phase-host stand-in: not an exposed skip-phase command.
        if stage > 1 {
            let cmd = command(
                &format!("phase-{stage}"),
                game.state().revision,
                "axis",
                InitiativeCommand::Determine {},
            );
            let result = game.reduce("axis", &cmd, |_, _, tx| {
                tx.game.calendar = Calendar::new(1, stage).unwrap();
                Ok(())
            });
            assert!(matches!(result, Outcome::Accepted { .. }));
        }
        let before = serde_json::to_vec(&game).unwrap();
        let wrong = command(
            "wrong-side",
            game.state().revision,
            "cw",
            InitiativeCommand::ChooseOrder { first },
        );
        assert_eq!(execute(&mut game, &rules, &wrong), Unavailable.outcome());
        assert_eq!(serde_json::to_vec(&game).unwrap(), before);
        let choice = command(
            &format!("choose-{stage}"),
            game.state().revision,
            "axis",
            InitiativeCommand::ChooseOrder { first },
        );
        let accepted = execute(&mut game, &rules, &choice);
        assert!(matches!(accepted, Outcome::Accepted { .. }));
        let expected_a = if first {
            Side::Axis
        } else {
            Side::Commonwealth
        };
        let expected_b = if first {
            Side::Commonwealth
        } else {
            Side::Axis
        };
        assert_eq!(
            game.events().last().unwrap().payload,
            InitiativeEvent::OrderDeclared {
                at: Calendar::new(1, stage).unwrap(),
                player_a: expected_a,
                player_b: expected_b,
            }
        );
        assert_eq!(
            game.state()
                .initiative
                .as_ref()
                .unwrap()
                .first_player(game.state().calendar),
            Some(expected_a)
        );
        let after = serde_json::to_vec(&game).unwrap();
        assert_eq!(execute(&mut game, &rules, &choice), accepted);
        let change = command(
            "change-choice",
            game.state().revision,
            "axis",
            InitiativeCommand::ChooseOrder { first: !first },
        );
        assert_eq!(execute(&mut game, &rules, &change), Unavailable.outcome());
        assert_eq!(serde_json::to_vec(&game).unwrap(), after);
    }
    assert_eq!(game.state().rng.word_position, 0);
    let mut next = game.state().clone();
    next.calendar = Calendar::new(2, 1).unwrap();
    assert_eq!(
        next.initiative
            .as_ref()
            .unwrap()
            .first_player(next.calendar),
        None
    );
    let mut next = session(next);
    let roll = command(
        "next-week",
        next.state().revision,
        "axis",
        InitiativeCommand::Determine {},
    );
    assert!(matches!(
        execute(&mut next, &rules, &roll),
        Outcome::Accepted { .. }
    ));
    assert_eq!(
        next.state().initiative.as_ref().unwrap().winner,
        Side::Commonwealth
    );
    assert_eq!(next.state().rng.word_position, 2);
}

#[test]
fn unresolved_references_wrong_stage_and_rng_exhaustion_leave_no_changes() {
    let rules = InitiativeRules {
        roles: BTreeMap::new(),
        opening: None,
    };
    for failure in 0..4 {
        let mut initial = position(1);
        match failure {
            0 => add(&mut initial, "unresolved", "A3033"),
            1 => initial.calendar = Calendar::new(1, 2).unwrap(),
            2 => initial.rng.word_position = u64::MAX - 1,
            _ => (),
        }
        let mut game = session(initial);
        let before = serde_json::to_vec(&game).unwrap();
        let order = if failure == 3 {
            InitiativeCommand::ChooseOrder { first: true }
        } else {
            InitiativeCommand::Determine {}
        };
        assert_eq!(
            execute(&mut game, &rules, &command("invalid", 0, "axis", order)),
            Unavailable.outcome()
        );
        assert_eq!(serde_json::to_vec(&game).unwrap(), before);
    }
    let mut initial = position(1);
    add(&mut initial, "rommel", "Tobruk Holding Box");
    let rules = InitiativeRules {
        roles: BTreeMap::from([(CanonicalUnitId("rommel".into()), InitiativeRole::Rommel)]),
        opening: None,
    };
    let mut game = session(initial);
    assert_eq!(
        execute(
            &mut game,
            &rules,
            &command("ambiguous-box", 0, "axis", InitiativeCommand::Determine {})
        ),
        Unavailable.outcome()
    );
    assert!(
        serde_json::from_str::<InitiativeCommand>(r#"{"kind":"determine","winner":"axis"}"#)
            .is_err()
    );
}

#[test]
fn midweek_scenario_state_round_trips_and_rejects_future_orders() {
    use cna_sim::official::initiative::InitiativeState;
    let at = Calendar::new(26, 3).unwrap();
    let rules = InitiativeRules {
        roles: BTreeMap::new(),
        opening: Some(OpeningInitiative {
            at,
            winner: Side::Commonwealth,
        }),
    };
    let mut initial = position(26);
    initial.calendar = at;
    let mut game = session(initial);
    assert!(matches!(
        execute(
            &mut game,
            &rules,
            &command("opening", 0, "cw", InitiativeCommand::Determine {})
        ),
        Outcome::Accepted { .. }
    ));
    assert!(matches!(
        execute(
            &mut game,
            &rules,
            &command(
                "choose",
                1,
                "cw",
                InitiativeCommand::ChooseOrder { first: false }
            )
        ),
        Outcome::Accepted { .. }
    ));
    assert_eq!(game.state().rng.word_position, 0);
    let restored: GameState =
        serde_json::from_slice(&serde_json::to_vec(game.state()).unwrap()).unwrap();
    assert_eq!(restored, *game.state());
    assert_eq!(
        restored.initiative.as_ref().unwrap().first_players,
        [None, None, Some(Side::Axis)]
    );
    for (determined_at, first_players) in [
        (Calendar::new(2, 1).unwrap(), [None; 3]),
        (Calendar::new(1, 2).unwrap(), [None; 3]),
        (Calendar::new(1, 1).unwrap(), [None, Some(Side::Axis), None]),
    ] {
        let mut invalid = position(1);
        invalid.initiative = Some(InitiativeState {
            determined_at,
            winner: Side::Axis,
            first_players,
        });
        assert!(invalid.validate(&BTreeSet::new()).is_err());
    }
    let mut invalid = restored;
    invalid.initiative.as_mut().unwrap().first_players[0] = Some(Side::Axis);
    assert!(invalid.validate(&BTreeSet::new()).is_err());
}
