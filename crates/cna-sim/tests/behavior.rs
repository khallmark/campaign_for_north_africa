use cna_sim::*;
use std::collections::HashSet;

#[test]
fn scenario_has_distinct_passable_positions_and_independent_state() {
    let mut state = new_game();
    assert_eq!(
        state
            .units
            .iter()
            .map(Unit::hex)
            .collect::<HashSet<_>>()
            .len(),
        7
    );
    for unit in &state.units {
        assert_ne!(terrain(unit.hex()), "sea");
    }
    state.units[0].hp = 0;
    assert_eq!(new_game().units[0].hp, 100);
}

#[test]
fn movement_charges_terrain_and_fuel_and_rejects_invalid_orders_without_mutations() {
    let mut state = new_game();
    state.units[0].r = 6;
    assert!(move_unit(&mut state, "7arm", Hex { c: 7, r: 6 }).ok);
    let tank = &state.units[0];
    assert_eq!((tank.c, tank.r, tank.cp, tank.fuel), (7, 6, 4, 92));
    assert!(move_unit(&mut state, "7arm", Hex { c: 6, r: 6 }).ok);
    let tank = &state.units[0];
    assert_eq!((tank.c, tank.r, tank.cp, tank.fuel), (6, 6, 2, 76));
    let before = state.clone();
    assert!(!move_unit(&mut state, "7arm", Hex { c: 6, r: 5 }).ok);
    assert!(!move_unit(&mut state, "7arm", Hex { c: 8, r: 0 }).ok);
    assert!(!move_unit(&mut state, "missing", Hex { c: 7, r: 6 }).ok);
    assert_eq!(state, before);
}

#[test]
fn fuel_and_water_constrain_travel_but_infantry_spends_no_fuel() {
    let mut state = new_game();
    state.units[0].fuel = 7;
    assert!(reachable(&state, Some(&state.units[0])).is_empty());
    state.units[0].fuel = 8;
    assert!(move_unit(&mut state, "7arm", Hex { c: 7, r: 5 }).ok);
    assert_eq!(state.units[0].fuel, 0);
    state.units[0].water = 0;
    assert!(reachable(&state, Some(&state.units[0])).is_empty());
    state.units[2].fuel = 0;
    assert!(move_unit(&mut state, "22inf", Hex { c: 11, r: 6 }).ok);
    assert_eq!(state.units[2].fuel, 0);
}

#[test]
fn enemy_control_stops_paths_and_players_cannot_move_axis() {
    let mut state = new_game();
    state.units[0].cp = 2;
    let moves = reachable(&state, Some(&state.units[0]));
    assert_eq!(moves.get(&Hex { c: 7, r: 5 }), Some(&1));
    assert!(!moves.contains_key(&Hex { c: 7, r: 4 }));
    let mut without_enemies = state.clone();
    without_enemies.units.retain(|u| u.side == "allied");
    assert_eq!(
        reachable(&without_enemies, Some(&without_enemies.units[0])).get(&Hex { c: 7, r: 4 }),
        Some(&2)
    );
    let before = state.clone();
    assert!(!move_unit(&mut state, "15pz", Hex { c: 7, r: 5 }).ok);
    assert_eq!(state, before);
}

#[test]
fn assault_spends_cp_and_ammo_and_only_surviving_defenders_return_fire() {
    let mut state = new_game();
    assert!(!attack(&mut state, "7arm", "15pz", "allied").ok);
    move_unit(&mut state, "7arm", Hex { c: 7, r: 5 });
    state.units[0].ammo = 19;
    let before = state.clone();
    assert!(!attack(&mut state, "7arm", "15pz", "allied").ok);
    assert_eq!(state, before);
    state.units[0].ammo = 100;
    assert!(attack(&mut state, "7arm", "15pz", "allied").ok);
    let (a, b) = (&state.units[0], &state.units[4]);
    assert_eq!((a.hp, b.hp, a.cp, a.ammo, b.ammo), (82, 60, 2, 80, 90));
    state.units[4].hp = 8;
    assert!(attack(&mut state, "7arm", "15pz", "allied").ok);
    let (a, b) = (&state.units[0], &state.units[4]);
    assert_eq!((a.hp, b.hp, a.cp, a.ammo), (82, 0, 0, 60));
    let before = state.clone();
    assert!(!attack(&mut state, "7arm", "15pz", "allied").ok);
    assert_eq!(state, before);
}

#[test]
fn digging_in_reduces_damage_does_not_stack_and_moving_removes_cover() {
    let mut state = new_game();
    move_unit(&mut state, "7arm", Hex { c: 7, r: 5 });
    assert!(dig_in(&mut state, "7arm").ok);
    assert!(!dig_in(&mut state, "7arm").ok);
    assert!(attack(&mut state, "15pz", "7arm", "axis").ok);
    assert_eq!(state.units[0].hp, 70);
    assert!(move_unit(&mut state, "7arm", Hex { c: 8, r: 5 }).ok);
    assert!(!state.units[0].entrenched);
}

#[test]
fn trucks_transfer_shortages_within_range_and_depot_refills_trucks() {
    let mut state = new_game();
    let tank = &mut state.units[1];
    tank.fuel = 80;
    tank.water = 40;
    tank.ammo = 95;
    assert!(resupply(&mut state, "4arm").ok);
    let tank = &state.units[1];
    assert_eq!(
        (tank.fuel, tank.water, tank.ammo, tank.cp),
        (100, 65, 100, 4)
    );
    let truck = &state.units[3];
    assert_eq!((truck.fuel, truck.water, truck.ammo), (80, 75, 95));
    state.units[0].fuel = 80;
    let before = state.clone();
    assert!(!resupply(&mut state, "7arm").ok);
    assert_eq!(state, before);
    let truck = &mut state.units[3];
    truck.c = 12;
    truck.r = 7;
    truck.fuel = 8;
    truck.water = 10;
    truck.ammo = 20;
    assert!(resupply(&mut state, "supply").ok);
    let truck = &state.units[3];
    assert_eq!(
        (truck.fuel, truck.water, truck.ammo, truck.cp),
        (100, 100, 100, 4)
    );
}

#[test]
fn stage_executes_enemy_actions_upkeep_weather_and_restores_cp() {
    let mut state = new_game();
    dig_in(&mut state, "7arm");
    end_stage(&mut state);
    assert_eq!(state.stage, 2);
    assert!(state.units[0].hp < 100, "Axis should close and attack");
    assert_eq!(state.units[0].cp, 5);
    assert_eq!((state.units[0].water, state.units[0].fuel), (90, 98));
    // Equal-distance candidates retain the prototype's row-major tie-break.
    assert_eq!(state.units[4].hex(), Hex { c: 7, r: 5 });
    end_stage(&mut state);
    assert_eq!(state.stage, 3);
    assert_eq!((state.units[0].water, state.units[0].fuel), (72, 93));
    assert_eq!(date_label(1), "18 Nov 1941");
    assert_eq!(date_label(4), "25 Nov 1941");
    assert_eq!(date_label(7), "2 Dec 1941");
    assert_eq!(date_label(20), "1 Jan 1942");
}

#[test]
fn dehydration_prevents_assault_and_causes_upkeep_losses() {
    let mut state = new_game();
    move_unit(&mut state, "7arm", Hex { c: 7, r: 5 });
    state.units[0].water = 0;
    assert!(!attack(&mut state, "7arm", "15pz", "allied").ok);
    state.units.retain(|u| u.side == "allied");
    end_stage(&mut state);
    assert_eq!(state.units[0].hp, 90);
}

#[test]
fn capture_wins_trucks_cannot_capture_and_losses_or_deadline_end_play() {
    let mut state = new_game();
    state.units[6].hp = 0;
    state.units[0].c = 4;
    state.units[0].r = 3;
    assert!(move_unit(&mut state, "7arm", TOBRUK).ok);
    assert_eq!(state.outcome.as_deref(), Some("victory"));
    let before = state.clone();
    assert!(!end_stage(&mut state).ok);
    assert!(!dig_in(&mut state, "7arm").ok);
    assert_eq!(state, before);

    let mut truck_only = new_game();
    truck_only.units[6].hp = 0;
    truck_only.units[3].c = TOBRUK.c;
    truck_only.units[3].r = TOBRUK.r;
    assert_eq!(check_outcome(&mut truck_only), None);
    for unit in truck_only
        .units
        .iter_mut()
        .filter(|u| u.side == "allied" && u.unit_type != "truck")
    {
        unit.hp = 0;
    }
    assert_eq!(check_outcome(&mut truck_only).as_deref(), Some("defeat"));
    let mut deadline = new_game();
    deadline.units.retain(|u| u.side == "allied");
    deadline.stage = STAGE_LIMIT;
    assert_eq!(check_outcome(&mut deadline), None);
    end_stage(&mut deadline);
    assert_eq!(deadline.outcome.as_deref(), Some("defeat"));
}
