mod official {
    mod reducer {
        use cna_sim::{
            contract::command::{Command, Event, Outcome, RandomDraw, SchemaVersion},
            official::{
                reducer::{noop, Noop, Session, Transaction, Unavailable},
                rng::RecordedRng,
                state::*,
            },
        };
        use serde::{Deserialize, Serialize};
        use sha2::{Digest, Sha256};
        use std::{collections::BTreeMap, num::NonZeroU32};

        fn initial(seed: u8, word_position: u64, revision: u64) -> GameState {
            GameState {
                revision,
                calendar: Calendar::new(1, 1).unwrap(),
                mode: Mode::DetailedLogisticsAbstractAir58,
                initiative: None,
                instances: BTreeMap::new(),
                organizations: BTreeMap::new(),
                cargo_batches: BTreeMap::from([(
                    "private-stock".into(),
                    CargoBatch {
                        side: Side::Axis,
                        holder: CargoHolder::Location("Tripoli".parse().unwrap()),
                        supplies: Supplies {
                            water: WaterPoints::whole(u32::from(seed)),
                            ..Supplies::default()
                        },
                    },
                )]),
                pending_commitments: BTreeMap::new(),
                rng: RngState {
                    seed: [seed; 32],
                    word_position,
                },
                references: ReferenceDigests {
                    ruleset_id: "fixture-only".into(),
                    ruleset: format!("sha256:{}", "1".repeat(64)),
                    map: format!("sha256:{}", "2".repeat(64)),
                    catalog: format!("sha256:{}", "3".repeat(64)),
                },
            }
        }

        fn session<P: Clone + Eq, E: Clone>(game: GameState) -> Session<P, E> {
            Session::new(
                game,
                BTreeMap::from([("player-a".into(), Side::Axis)]),
                Default::default(),
            )
            .unwrap()
        }

        fn command<P>(id: &str, revision: u64, payload: P) -> Command<P> {
            Command {
                schema_version: SchemaVersion::V1,
                ruleset_id: "fixture-only".into(),
                ruleset_digest: format!("sha256:{}", "1".repeat(64)),
                actor: "player-a".into(),
                game_revision: revision,
                command_id: id.into(),
                payload,
            }
        }

        fn bytes(value: &impl Serialize) -> Vec<u8> {
            serde_json::to_vec(value).unwrap()
        }

        fn digest(value: &impl Serialize) -> Vec<u8> {
            Sha256::digest(bytes(value)).to_vec()
        }

        // Synthetic kernel exercises the transaction contract; these are NOT game orders.
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Probe {
            bounds: Vec<u32>,
            reject: bool,
            extensions: BTreeMap<String, String>,
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
        struct Roll {
            value: u32,
        }

        fn probe(bounds: &[u32], reject: bool) -> Probe {
            Probe {
                bounds: bounds.into(),
                reject,
                extensions: BTreeMap::new(),
            }
        }

        fn kernel(
            side: Side,
            action: &Probe,
            tx: &mut Transaction<Roll>,
        ) -> Result<(), Unavailable> {
            assert_eq!(side, Side::Axis);
            tx.game
                .cargo_batches
                .get_mut("private-stock")
                .unwrap()
                .supplies
                .water = WaterPoints::whole(777);
            for &bound in &action.bounds {
                tx.emit(|rng| {
                    Ok(Roll {
                        value: rng.bounded(NonZeroU32::new(bound).ok_or(Unavailable)?)?,
                    })
                })?;
            }
            if action.reject {
                Err(Unavailable)
            } else {
                Ok(())
            }
        }

        #[test]
        fn official_noop_is_idempotent_without_advancing_calendar_or_rng() {
            let initial = initial(9, 19, 0);
            let mut session = Session::<Noop, Noop>::new(
                initial.clone(),
                BTreeMap::from([
                    ("player-a".into(), Side::Axis),
                    ("player-b".into(), Side::Commonwealth),
                ]),
                Default::default(),
            )
            .unwrap();
            let first = command("first", 0, Noop {});
            let expected = Outcome::Accepted {
                events: vec![Event {
                    sequence: 0,
                    command_id: "first".into(),
                    payload: Noop {},
                    random_draws: vec![],
                }],
                new_revision: 1,
            };
            assert_eq!(session.reduce("player-a", &first, noop), expected);
            let mut revised = initial;
            revised.revision = 1;
            assert_eq!(session.state(), &revised);
            let accepted = bytes(&session);
            assert_eq!(session.reduce("player-a", &first, noop), expected);
            assert_eq!(bytes(&session), accepted);
            assert_eq!(
                session.reduce("another-player", &first, noop),
                Unavailable.outcome()
            );
            assert_eq!(bytes(&session), accepted);
            assert_eq!(
                session.reduce("player-b", &first, noop),
                Unavailable.outcome()
            );
            assert_eq!(bytes(&session), accepted);
            let mut second = command("second", 1, Noop {});
            second.actor = "player-b".into();
            let outcome = session.reduce("player-b", &second, |side, payload, tx| {
                if side != Side::Commonwealth {
                    return Err(Unavailable);
                }
                noop(side, payload, tx)
            });
            assert!(matches!(
                outcome,
                Outcome::Accepted {
                    new_revision: 2,
                    ..
                }
            ));
            assert_eq!(session.events()[1].sequence, 1);
            assert!(serde_json::from_str::<Noop>(r#"{"move":"A3033"}"#).is_err());
        }

        #[test]
        fn official_named_trace_records_rejection_sampling_and_global_event_order() {
            let trace = [
                command("sample", 0, probe(&[0x8000_0001, 6], false)),
                command("rejected", 1, probe(&[7, 13], true)),
                command("sample", 0, probe(&[0x8000_0001, 6], false)),
                command("empty", 1, probe(&[], false)),
                command("next", 2, probe(&[1], false)),
            ];
            let run = |annotated| {
                let mut session = session::<Probe, Roll>(initial(0, 0, 0));
                for cmd in &trace {
                    let mut cmd = cmd.clone();
                    if annotated {
                        cmd.payload
                            .extensions
                            .insert("note".into(), "non-semantic".into());
                    }
                    session.reduce("player-a", &cmd, kernel);
                }
                session
            };
            let first = run(false);
            let second = run(false);
            let annotated = run(true);
            // Values derive from the independent TC1 ChaCha8 vector, not rand_chacha.
            let expected = vec![
                Event {
                    sequence: 0,
                    command_id: "sample".into(),
                    payload: Roll { value: 0x5640_5f88 },
                    random_draws: vec![
                        RandomDraw {
                            word_index: 0,
                            value: 0x2fef_003e,
                        },
                        RandomDraw {
                            word_index: 1,
                            value: 0xd640_5f89,
                        },
                    ],
                },
                Event {
                    sequence: 1,
                    command_id: "sample".into(),
                    payload: Roll { value: 1 },
                    random_draws: vec![RandomDraw {
                        word_index: 2,
                        value: 0xe8b8_5b7f,
                    }],
                },
                Event {
                    sequence: 2,
                    command_id: "next".into(),
                    payload: Roll { value: 0 },
                    random_draws: vec![RandomDraw {
                        word_index: 3,
                        value: 0xa1a5_091f,
                    }],
                },
            ];
            assert_eq!(first.events(), expected);
            assert_eq!(digest(&first.events()), digest(&expected));
            assert_eq!(bytes(&first), bytes(&second));
            assert_eq!(first.events(), annotated.events());
            assert_eq!(first.state(), annotated.state());
            assert_eq!(first.state().rng.word_position, 4);
            assert_eq!(first.state().revision, 3);
            assert_eq!(
                first.state().cargo_batches["private-stock"].supplies.water,
                WaterPoints::whole(777)
            );
        }

        #[test]
        fn official_rejection_rollback_property_over_seeds_cursors_and_failure_points() {
            // Exhaustive bounded property: 256 distinct private stocks/seeds × 10 failures.
            // Every failure preserves the ENTIRE session, not just its visible fields.
            let public_rejection = bytes(&Unavailable.outcome::<Roll>());
            for seed in 0..=u8::MAX {
                for failure in 0..10 {
                    let mut session =
                        session::<Probe, Roll>(initial(seed, u64::from(seed) * 17, 3));
                    let mut cmd = command("retryable", 3, probe(&[7, 13], false));
                    let mut principal = "player-a";
                    match failure {
                        0 => cmd.command_id = " ".into(),
                        1 => cmd.ruleset_id = "other".into(),
                        2 => cmd.ruleset_digest = format!("sha256:{}", "4".repeat(64)),
                        3 => cmd.ruleset_digest = "bad-digest".into(),
                        4 => cmd.game_revision = 2,
                        5 => principal = "observer",
                        6 => {
                            principal = "observer";
                            cmd.actor = "observer".into();
                        }
                        7 => cmd.actor = "observer".into(),
                        8 => cmd.payload.reject = true,
                        9 => cmd.payload.bounds = vec![7, 0],
                        _ => unreachable!(),
                    }
                    let before = bytes(&session);
                    let outcome = session.reduce(principal, &cmd, kernel);
                    assert_eq!(
                        bytes(&outcome),
                        public_rejection,
                        "seed {seed}, failure {failure}"
                    );
                    assert_eq!(bytes(&session), before, "seed {seed}, failure {failure}");
                    // A rejected ID is not retained; a corrected retry must still run.
                    let good = command("retryable", 3, probe(&[7], false));
                    assert!(matches!(
                        session.reduce("player-a", &good, kernel),
                        Outcome::Accepted {
                            new_revision: 4,
                            ..
                        }
                    ));
                }
            }
        }

        #[test]
        fn official_receipts_compare_complete_commands_and_precede_stale_revision_rejection() {
            let mut session = session::<Probe, Roll>(initial(7, 17, 0));
            let cmd = command("original", 0, probe(&[6], false));
            let accepted = session.reduce("player-a", &cmd, kernel);
            session.reduce("player-a", &command("later", 1, probe(&[8], false)), kernel);
            let before = bytes(&session);
            assert_eq!(session.reduce("player-a", &cmd, kernel), accepted);
            for field in 0..5 {
                let mut changed = cmd.clone();
                match field {
                    0 => changed.game_revision = 2,
                    1 => changed.payload.bounds = vec![8],
                    2 => {
                        changed
                            .payload
                            .extensions
                            .insert("note".into(), "different".into());
                    }
                    3 => changed.ruleset_id = "other".into(),
                    4 => changed.actor = "observer".into(),
                    _ => unreachable!(),
                }
                assert_eq!(
                    session.reduce("player-a", &changed, kernel),
                    Unavailable.outcome()
                );
                assert_eq!(bytes(&session), before);
            }
        }

        #[test]
        fn official_overflows_and_invalid_kernel_state_roll_back() {
            let mut invalid = initial(0, 0, 0);
            invalid.references.ruleset = "invalid-digest".into();
            assert!(
                Session::<Probe, Roll>::new(invalid, Default::default(), Default::default())
                    .is_err()
            );
            for (revision, cursor) in [(u64::MAX, 0), (0, u64::MAX - 1), (0, u64::MAX)] {
                let mut session = session::<Probe, Roll>(initial(2, cursor, revision));
                let before = bytes(&session);
                let cmd = command("exhaust", revision, probe(&[1, 1], false));
                assert_eq!(
                    session.reduce("player-a", &cmd, kernel),
                    Unavailable.outcome()
                );
                assert_eq!(bytes(&session), before);
            }
            for change in 0..4 {
                let mut session = session::<Probe, Roll>(initial(0, 0, 0));
                let before = bytes(&session);
                let result = session.reduce(
                    "player-a",
                    &command("bad-kernel", 0, probe(&[], false)),
                    |_, _, tx| {
                        match change {
                            0 => tx.game.revision += 1,
                            1 => tx.game.rng.word_position += 1,
                            2 => tx.game.references.ruleset_id = "other".into(),
                            3 => {
                                tx.game
                                    .cargo_batches
                                    .get_mut("private-stock")
                                    .unwrap()
                                    .holder = CargoHolder::Instance("missing".into())
                            }
                            _ => unreachable!(),
                        }
                        Ok(())
                    },
                );
                assert_eq!(result, Unavailable.outcome());
                assert_eq!(bytes(&session), before);
            }
            for cursor in [0, u64::MAX - 1] {
                let mut session = session::<Probe, Roll>(initial(2, cursor, 0));
                let before = bytes(&session);
                let result = session.reduce(
                    "player-a",
                    &command("ignored-error", 0, probe(&[], false)),
                    |_, _, tx| {
                        tx.emit(|rng| {
                            Ok(Roll {
                                value: rng.next_word()?,
                            })
                        })?;
                        // Neither ignoring a kernel error nor ignoring an exhausted RNG
                        // inside an event may turn a partial transaction into acceptance.
                        let _ = tx.emit(|rng| {
                            if cursor == 0 {
                                return Err(Unavailable);
                            }
                            let _ = rng.next_word();
                            Ok(Roll { value: 0 })
                        });
                        Ok(())
                    },
                );
                assert_eq!(result, Unavailable.outcome());
                assert_eq!(bytes(&session), before);
            }
        }

        #[test]
        fn official_rng_matches_independent_chacha8_vectors_and_resumes_across_blocks() {
            // Strombergson draft -01 §3 TC1, 256-bit key, 8 rounds, blocks 0 and 1.
            // https://www.ietf.org/archive/id/draft-strombergson-chacha-test-vectors-01.txt
            let golden = [
                0x2fef003e, 0xd6405f89, 0xe8b85b7f, 0xa1a5091f, 0xc30e842c, 0x3b7f9ace, 0x88e11b18,
                0x1e1a71ef, 0x72e14c98, 0x416f21b9, 0x6753449f, 0x19566d45, 0xa3424a31, 0x01b086da,
                0xb8fd7b38, 0x42fe0c0e, 0x0dfaaed2, 0x51c1a5ea, 0x6cdb0abf, 0xada5f201, 0x1258fdc0,
                0xaaa2f959, 0x8f0ff2dc, 0x6ba266d5, 0x38ec3250, 0x98dac5bb, 0x566f0cee, 0x652a878b,
                0x25bf8aa0, 0xbb21eb1d, 0xd8e5564b, 0xaa681e82,
            ];
            for start in [0, 1, 15, 16, 17, 31] {
                let mut rng = RecordedRng::new(&RngState {
                    seed: [0; 32],
                    word_position: start as u64,
                });
                for (index, &expected) in golden.iter().enumerate().skip(start) {
                    assert_eq!(rng.next_word().unwrap(), expected);
                    assert_eq!(
                        rng.draws().last(),
                        Some(&RandomDraw {
                            word_index: index as u64,
                            value: expected
                        })
                    );
                }
                assert_eq!(rng.state().word_position, 32);
            }
            // TC2 sets the first key byte to 1, exposing ignored seeds and byte order.
            let mut seed = [0; 32];
            seed[0] = 1;
            let mut rng = RecordedRng::new(&RngState {
                seed,
                word_position: 0,
            });
            for word in [0xa0e95ecf, 0x61a94a49, 0xedd5053e, 0x4b805b72] {
                assert_eq!(rng.next_word().unwrap(), word);
            }
            for bound in [0x8000_0000, u32::MAX] {
                let mut rng = RecordedRng::new(&RngState {
                    seed: [0; 32],
                    word_position: 0,
                });
                assert_eq!(
                    rng.bounded(NonZeroU32::new(bound).unwrap()).unwrap(),
                    0x2fef003e
                );
                assert_eq!(rng.state().word_position, 1);
            }
            let mut exhausted = RecordedRng::new(&RngState {
                seed: [0; 32],
                word_position: u64::MAX,
            });
            assert_eq!(exhausted.next_word(), Err(Unavailable));
            assert!(exhausted.draws().is_empty());
            assert_eq!(exhausted.state().word_position, u64::MAX);
        }
    }
}
