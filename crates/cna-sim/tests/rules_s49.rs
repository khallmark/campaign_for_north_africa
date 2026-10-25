//! Arithmetic fixtures from AL21 p21 §49.13/.14/.3 and R79 p67.
//! These do not claim routing, supply access or phase execution.
use cna_sim::{
    contract::route::Points,
    official::{
        fuel::{
            consumption, draw, evaporate, tank_capacity, FuelAction, FuelBasis, FuelStorage,
            LossPeriod,
        },
        state::{Calendar, FuelPoints, Side, ToePoints},
    },
};
use std::num::NonZeroU32;

fn cp(numerator: u32, denominator: u32) -> Points {
    Points {
        numerator,
        denominator: NonZeroU32::new(denominator).unwrap(),
    }
}

#[test]
fn movement_fuel_uses_each_started_five_cp_group_per_toe_not_combat_cp() {
    // The printed example: one TOE × rate 4 × ceil(12/5) = 12 Fuel Points.
    for (numerator, denominator, expected) in [
        (0, 1, 0),
        (1, 2, 4),
        (5, 1, 4),
        (11, 2, 8),
        (10, 1, 8),
        (21, 2, 12),
        (12, 1, 12),
    ] {
        assert_eq!(
            consumption(
                ToePoints::whole(1),
                FuelPoints::whole(4),
                FuelAction::Movement {
                    cp: cp(numerator, denominator)
                },
                FuelBasis::StartedFiveCpGroups
            )
            .unwrap(),
            FuelPoints::whole(expected)
        );
    }
    assert_eq!(
        consumption(
            ToePoints::new(3, 2).unwrap(),
            FuelPoints::whole(4),
            FuelAction::Movement { cp: cp(12, 1) },
            FuelBasis::StartedFiveCpGroups
        )
        .unwrap(),
        FuelPoints::whole(18)
    );
    assert_eq!(
        consumption(
            ToePoints::whole(20),
            FuelPoints::whole(4),
            FuelAction::Stationary,
            FuelBasis::StartedFiveCpGroups
        )
        .unwrap(),
        FuelPoints::whole(0)
    );
    // 49.14 capacity is the printed CPA/5 × rate × TOE formula, not a stock refill.
    assert_eq!(
        tank_capacity(ToePoints::whole(2), FuelPoints::whole(4), cp(25, 1)).unwrap(),
        FuelPoints::whole(40)
    );
    assert_eq!(
        tank_capacity(ToePoints::whole(1), FuelPoints::whole(4), cp(12, 1)).unwrap(),
        FuelPoints::new(48, 5).unwrap()
    );
    assert_eq!(
        tank_capacity(
            ToePoints::new(3, 2).unwrap(),
            FuelPoints::whole(1),
            cp(25, 1)
        )
        .unwrap(),
        FuelPoints::new(15, 2).unwrap()
    );
}

#[test]
fn fuel_debits_are_exact_and_fail_without_mutation_when_stock_or_range_is_insufficient() {
    let mut stock = FuelPoints::new(13, 2).unwrap();
    draw(&mut stock, FuelPoints::new(7, 3).unwrap()).unwrap();
    assert_eq!(stock, FuelPoints::new(25, 6).unwrap());
    let before = stock;
    assert!(draw(&mut stock, FuelPoints::new(17, 4).unwrap()).is_err());
    assert_eq!(stock, before);
    draw(&mut stock, FuelPoints::new(50, 12).unwrap()).unwrap();
    assert_eq!(stock, FuelPoints::whole(0));
    let mut noncanonical = FuelPoints::new(2, 4).unwrap();
    draw(&mut noncanonical, FuelPoints::whole(0)).unwrap();
    assert_eq!(noncanonical, FuelPoints::new(2, 4).unwrap());
    let mut too_precise = FuelPoints::new(1, u32::MAX - 1).unwrap();
    let before = too_precise;
    assert!(draw(&mut too_precise, FuelPoints::new(1, u32::MAX).unwrap()).is_err());
    assert_eq!(too_precise, before);
    // Large intermediate products may reduce to a representable exact result.
    assert_eq!(
        consumption(
            ToePoints::new(u32::MAX, u32::MAX).unwrap(),
            FuelPoints::whole(1),
            FuelAction::Movement { cp: cp(1, 1) },
            FuelBasis::StartedFiveCpGroups
        )
        .unwrap(),
        FuelPoints::whole(1)
    );
    assert!(consumption(
        ToePoints::whole(u32::MAX),
        FuelPoints::whole(2),
        FuelAction::Movement { cp: cp(1, 1) },
        FuelBasis::StartedFiveCpGroups
    )
    .is_err());
}

#[test]
fn original_chart_conflict_is_an_explicit_basis_not_a_silent_rounding_choice() {
    // Original R79 p106 chart §49.19, visually compared to the VASSAL chart.
    // Rate 4 at CP 1,2,3,4,5 gives .8,1.6,2.4,3.2,4, contrary to §49.13.
    for (points, numerator) in [(1, 4_u32), (2, 8), (3, 12), (4, 16), (5, 20)] {
        let value = consumption(
            ToePoints::whole(1),
            FuelPoints::whole(4),
            FuelAction::Movement { cp: cp(points, 1) },
            FuelBasis::ProportionalCp,
        )
        .unwrap();
        // Cross-multiply since the public result is reduced exactly.
        assert_eq!(
            u64::from(value.0.numerator) * 5,
            u64::from(numerator) * u64::from(value.0.denominator.get())
        );
    }
    assert_eq!(
        consumption(
            ToePoints::whole(1),
            FuelPoints::whole(4),
            FuelAction::Movement { cp: cp(12, 1) },
            FuelBasis::ProportionalCp
        )
        .unwrap(),
        FuelPoints::new(48, 5).unwrap()
    );
}

#[test]
fn weekly_loss_changes_after_august_1941_and_hot_loss_uses_remaining_on_map_stock() {
    // GT46 = August IV 1941, GT47 = September I 1941; boundary is inclusive.
    for (side, turn, expected_loss) in [
        (Side::Axis, 1, 6),
        (Side::Axis, 46, 6),
        (Side::Commonwealth, 46, 9),
        (Side::Commonwealth, 47, 6),
        (Side::Commonwealth, 111, 6),
    ] {
        let mut stock = FuelPoints::whole(101);
        assert_eq!(
            evaporate(
                &mut stock,
                side,
                Calendar::new(turn, 1).unwrap(),
                LossPeriod::Weekly,
                FuelStorage::OnMap
            )
            .unwrap(),
            FuelPoints::whole(expected_loss)
        );
        assert_eq!(stock, FuelPoints::whole(101 - expected_loss));
    }
    let mut stock = FuelPoints::whole(100);
    evaporate(
        &mut stock,
        Side::Axis,
        Calendar::new(1, 1).unwrap(),
        LossPeriod::Weekly,
        FuelStorage::OnMap,
    )
    .unwrap();
    assert_eq!(stock, FuelPoints::whole(94));
    assert_eq!(
        evaporate(
            &mut stock,
            Side::Axis,
            Calendar::new(1, 1).unwrap(),
            LossPeriod::HotWeather,
            FuelStorage::OnMap
        )
        .unwrap(),
        FuelPoints::whole(4)
    );
    assert_eq!(stock, FuelPoints::whole(90));
    for storage in [FuelStorage::ConvoyAtSea, FuelStorage::OffMap] {
        let before = stock;
        assert_eq!(
            evaporate(
                &mut stock,
                Side::Commonwealth,
                Calendar::new(1, 1).unwrap(),
                LossPeriod::Weekly,
                storage
            )
            .unwrap(),
            FuelPoints::whole(0)
        );
        assert_eq!(stock, before);
    }
    // Round the amount lost DOWN, not the amount remaining or to nearest .5.
    for (quantity, expected_loss) in [(16, 0), (17, 1), (25, 1), (33, 1), (34, 2)] {
        let mut stock = FuelPoints::whole(quantity);
        assert_eq!(
            evaporate(
                &mut stock,
                Side::Axis,
                Calendar::new(1, 1).unwrap(),
                LossPeriod::Weekly,
                FuelStorage::OnMap
            )
            .unwrap(),
            FuelPoints::whole(expected_loss)
        );
        assert_eq!(stock, FuelPoints::whole(quantity - expected_loss));
    }
}

#[test]
fn fuel_being_consumed_is_exempt_from_hot_loss_but_not_weekly_spillage() {
    // R79 p41 / L21 p43 §29.34 excludes gas actually being consumed in tanks
    // from hot-weather loss. §49.3 weekly loss still applies regardless of storage.
    for (storage, period, side, expected_loss) in [
        (FuelStorage::OnMap, LossPeriod::HotWeather, Side::Axis, 5),
        (
            FuelStorage::OnMapBeingConsumed,
            LossPeriod::HotWeather,
            Side::Axis,
            0,
        ),
        (
            FuelStorage::OnMapBeingConsumed,
            LossPeriod::Weekly,
            Side::Axis,
            6,
        ),
        (
            FuelStorage::OnMapBeingConsumed,
            LossPeriod::Weekly,
            Side::Commonwealth,
            9,
        ),
    ] {
        let mut stock = FuelPoints::whole(101);
        let lost = evaporate(
            &mut stock,
            side,
            Calendar::new(1, 1).unwrap(),
            period,
            storage,
        )
        .unwrap();
        assert_eq!(lost, FuelPoints::whole(expected_loss));
        assert_eq!(stock, FuelPoints::whole(101 - expected_loss));
    }
}
