use cna_sim::{
    official::state::{Calendar, FuelPoints, Side, Supplies},
    tabletop::{Game, Order, Piece, Setup},
};

fn game() -> Game {
    Game::new(Setup {
        name: "Integration fixture — no historical deployment claim".into(),
        calendar: Calendar::new(1, 3).unwrap(),
        first: Side::Axis,
        seed: 97,
        pieces: [(1, Side::Axis, "C4020"), (2, Side::Commonwealth, "C3922")]
            .map(|(id, side, hex)| Piece {
                id,
                identity: format!("fixture-{id}"),
                side,
                hex: hex.parse().unwrap(),
                supplies: None,
                eliminated: false,
                dug_in: false,
            })
            .to_vec(),
    })
    .unwrap()
}

#[test]
fn hotseat_round_trip_keeps_orders_exact_stock_dice_and_finished_result() {
    let mut game = game();
    game.apply(Order::Supplies {
        id: 1,
        supplies: Supplies {
            fuel: FuelPoints::new(7, 2).unwrap(),
            ..Supplies::default()
        },
        note: "Setup ledger".into(),
    })
    .unwrap();
    game.apply(Order::Move {
        id: 1,
        to: "C4021".parse().unwrap(),
        fuel: Some(FuelPoints::new(2, 3).unwrap()),
        note: "Player-adjudicated movement".into(),
    })
    .unwrap();
    assert_eq!(
        game.pieces[0].supplies.unwrap().fuel,
        FuelPoints::new(17, 6).unwrap()
    );
    assert_eq!(game.pieces[0].hex.to_string(), "C4021");
    game.apply(Order::Roll {}).unwrap();
    let mut restored = Game::load(&game.save()).unwrap();
    assert_eq!(restored, game);
    game.apply(Order::Roll {}).unwrap();
    restored.apply(Order::Roll {}).unwrap();
    assert_eq!(restored, game);
    game.apply(Order::EndTurn {
        note: "Axis phases complete".into(),
    })
    .unwrap();
    assert_eq!(game.active, Side::Commonwealth);
    assert_eq!(game.calendar, Calendar::new(1, 3).unwrap());
    game.apply(Order::DigIn {
        id: 2,
        note: "Construction adjudicated".into(),
    })
    .unwrap();
    game.apply(Order::EndTurn {
        note: "Commonwealth phases complete".into(),
    })
    .unwrap();
    assert!(game.awaiting_stage);
    game.apply(Order::BeginStage {
        first: Side::Commonwealth,
        note: "New week: initiative, convoy and preliminary phases adjudicated".into(),
    })
    .unwrap();
    assert_eq!(game.calendar, Calendar::new(2, 1).unwrap());
    assert_eq!(game.active, Side::Commonwealth);
    game.apply(Order::Combat {
        attacker: 2,
        defender: 1,
        attacker_lost: false,
        defender_lost: true,
        note: "Result adjudicated from rulebook".into(),
    })
    .unwrap();
    assert!(game.pieces[0].eliminated);
    assert!(!game.pieces[1].eliminated);
    assert_eq!(
        game.winner, None,
        "Elimination is not a scenario victory check"
    );
    game.apply(Order::EndTurn {
        note: "Phases complete".into(),
    })
    .unwrap();
    game.apply(Order::Concede {}).unwrap();
    assert_eq!(game.winner, Some(Side::Commonwealth));
    assert_eq!(Game::load(&game.save()).unwrap(), game);
    let before = game.clone();
    assert!(game.apply(Order::Roll {}).is_err());
    assert_eq!(game, before);
}

#[test]
fn invalid_orders_and_saves_cannot_change_position_or_skip_both_players() {
    let mut game = game();
    for order in [
        Order::Move {
            id: 2,
            to: "C3921".parse().unwrap(),
            fuel: None,
            note: "Wrong side".into(),
        },
        Order::Move {
            id: 1,
            to: "C4021".parse().unwrap(),
            fuel: Some(FuelPoints::whole(1)),
            note: "Unknown stock".into(),
        },
        Order::Move {
            id: 1,
            to: "C9999".parse().unwrap(),
            fuel: None,
            note: "Outside geometry".into(),
        },
        Order::DigIn {
            id: 1,
            note: String::new(),
        },
        Order::BeginStage {
            first: Side::Axis,
            note: "Too soon".into(),
        },
        Order::Combat {
            attacker: 1,
            defender: 1,
            attacker_lost: true,
            defender_lost: true,
            note: "Own counter".into(),
        },
    ] {
        let before = game.clone();
        assert!(game.apply(order).is_err());
        assert_eq!(game, before);
    }
    game.apply(Order::Supplies {
        id: 1,
        supplies: Supplies {
            fuel: FuelPoints::whole(2),
            ..Supplies::default()
        },
        note: "Ledger".into(),
    })
    .unwrap();
    let before = game.clone();
    assert!(game
        .apply(Order::Move {
            id: 1,
            to: "C4021".parse().unwrap(),
            fuel: Some(FuelPoints::new(5, 2).unwrap()),
            note: "Insufficient stock".into()
        })
        .is_err());
    assert_eq!(game, before);
    let mut save: serde_json::Value = serde_json::from_str(&game.save()).unwrap();
    save["orders"][0]["id"] = 2.into();
    assert!(Game::load(&save.to_string()).is_err());
    game.apply(Order::EndTurn {
        note: "Complete".into(),
    })
    .unwrap();
    game.apply(Order::EndTurn {
        note: "Complete".into(),
    })
    .unwrap();
    let before = game.clone();
    assert!(game
        .apply(Order::EndTurn {
            note: "Cannot double end".into()
        })
        .is_err());
    assert_eq!(game, before);
}
