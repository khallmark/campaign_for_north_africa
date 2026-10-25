//! Land-fuel arithmetic, AL21 p21 §49.13/.14/.3. These are trusted-kernel
//! operations, not player commands: routing, stock access and phase cadence
//! must be checked by the calling rule. See docs/rules/fuel.md for the unresolved
//! original text/chart discrepancy; consumption deliberately has no default basis.
use super::{
    reducer::Unavailable,
    state::{Calendar, FuelPoints, Side, ToePoints},
};
use crate::contract::route::Points;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuelBasis {
    /// §49.13 prose and its 12-CP example, in both R79 and AL21.
    StartedFiveCpGroups,
    /// Original §49.19 chart's fractional 1–4 CP rows (R79 p106).
    ProportionalCp,
}

#[derive(Clone, Copy, Debug)]
pub enum FuelAction {
    Movement {
        cp: Points,
    },
    /// Combat and other land actions with no actual hex-to-hex movement.
    Stationary,
}

/// Compute fuel for one movement segment, not separately per route hex.
/// Rate is per TOE point per five CP; profile resolution belongs to the host.
pub fn consumption(
    toe: ToePoints,
    rate: FuelPoints,
    action: FuelAction,
    basis: FuelBasis,
) -> Result<FuelPoints, Unavailable> {
    let FuelAction::Movement { cp } = action else {
        return Ok(FuelPoints::whole(0));
    };
    let cp_n = u128::from(cp.numerator);
    let cp_d = u128::from(cp.denominator.get()) * 5;
    let (groups_n, groups_d) = match basis {
        FuelBasis::StartedFiveCpGroups => (cp_n.div_ceil(cp_d), 1),
        FuelBasis::ProportionalCp => (cp_n, cp_d),
    };
    exact(
        u128::from(toe.0.numerator) * u128::from(rate.0.numerator) * groups_n,
        u128::from(toe.0.denominator.get()) * u128::from(rate.0.denominator.get()) * groups_d,
    )
}

/// The literal §49.14 formula. Does not refill a tank or grant supply access.
pub fn tank_capacity(
    toe: ToePoints,
    rate: FuelPoints,
    cpa: Points,
) -> Result<FuelPoints, Unavailable> {
    exact(
        u128::from(toe.0.numerator) * u128::from(rate.0.numerator) * u128::from(cpa.numerator),
        u128::from(toe.0.denominator.get())
            * u128::from(rate.0.denominator.get())
            * u128::from(cpa.denominator.get())
            * 5,
    )
}

/// Caller first checks co-location, ownership and accessibility (§49.15/.16).
/// Insufficient stock or an unrepresentable result leaves the stock unchanged.
pub fn draw(stock: &mut FuelPoints, required: FuelPoints) -> Result<(), Unavailable> {
    if required.0.numerator == 0 {
        return Ok(());
    }
    let available = u128::from(stock.0.numerator) * u128::from(required.0.denominator.get());
    let cost = u128::from(required.0.numerator) * u128::from(stock.0.denominator.get());
    let remaining = available.checked_sub(cost).ok_or(Unavailable)?;
    *stock = exact(
        remaining,
        u128::from(stock.0.denominator.get()) * u128::from(required.0.denominator.get()),
    )?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LossPeriod {
    Weekly,
    HotWeather,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuelStorage {
    OnMap,
    ConvoyAtSea,
    OffMap,
}

/// Lose floor(stock × percentage) fuel points and return the amount lost.
/// Host applies Weekly once in Stores Expenditure and HotWeather once when
/// that stage's hot weather is determined; this arithmetic has no phase cursor.
pub fn evaporate(
    stock: &mut FuelPoints,
    side: Side,
    at: Calendar,
    period: LossPeriod,
    storage: FuelStorage,
) -> Result<FuelPoints, Unavailable> {
    if storage != FuelStorage::OnMap {
        return Ok(FuelPoints::whole(0));
    }
    let date = at.date();
    let percent = match period {
        LossPeriod::HotWeather => 5_u128,
        LossPeriod::Weekly
            if side == Side::Commonwealth
                && (date.year < 1941 || (date.year == 1941 && date.month <= 8)) =>
        {
            9
        }
        LossPeriod::Weekly => 6,
    };
    let lost =
        u128::from(stock.0.numerator) * percent / (u128::from(stock.0.denominator.get()) * 100);
    let lost = FuelPoints::whole(u32::try_from(lost).map_err(|_| Unavailable)?);
    draw(stock, lost)?;
    Ok(lost)
}

// All callers supply nonzero denominators and products of at most three u32
// quantities × 5, which fit u128. Reduce before narrowing to the state contract.
fn exact(numerator: u128, denominator: u128) -> Result<FuelPoints, Unavailable> {
    let (mut a, mut b) = (numerator, denominator);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    FuelPoints::new(
        u32::try_from(numerator / a).map_err(|_| Unavailable)?,
        u32::try_from(denominator / a).map_err(|_| Unavailable)?,
    )
    .map_err(|_| Unavailable)
}
