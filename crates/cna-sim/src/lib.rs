//! A small deterministic teaching scenario, not a rules-complete CNA engine.
//! All state and rules live here; the browser is only an input/rendering adapter.

use std::collections::HashMap;
pub mod contract;
mod roles;
pub use roles::{role, Domain, Role, Support, ROLES};

pub const COLS: i32 = 15;
pub const ROWS: i32 = 10;
pub const STAGE_LIMIT: i32 = 12;
pub const TOBRUK: Hex = Hex { c: 3, r: 3 };
pub const DEPOT: Hex = Hex { c: 13, r: 7 };
pub const COAST: &[(f64, f64)] = &[
    (0., 173.),
    (100., 183.),
    (180., 175.),
    (235., 205.),
    (280., 204.),
    (330., 191.),
    (395., 218.),
    (460., 239.),
    (535., 245.),
    (600., 273.),
    (647., 311.),
    (700., 304.),
    (745., 332.),
    (795., 375.),
    (860., 365.),
    (935., 348.),
    (1000., 351.),
    (1120., 318.),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hex {
    pub c: i32,
    pub r: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    pub id: String,
    /// Canonical catalog identity; absent for fictional prototype role units.
    pub identity_id: Option<String>,
    pub name: String,
    pub short: String,
    pub side: String,
    pub unit_type: String,
    pub c: i32,
    pub r: i32,
    pub hp: i32,
    pub cp: i32,
    pub max_cp: i32,
    pub fuel: i32,
    pub water: i32,
    pub ammo: i32,
    pub entrenched: bool,
}

impl Unit {
    pub fn role(&self) -> &'static Role {
        let id = if self.unit_type == "tank" {
            if self.side == "axis" {
                "panzer"
            } else {
                "crusader"
            }
        } else {
            &self.unit_type
        };
        role(id).expect("Every simulation unit has a known sandbox role")
    }

    pub fn hex(&self) -> Hex {
        Hex {
            c: self.c,
            r: self.r,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub stage: i32,
    pub text: String,
    pub log_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub stage: i32,
    pub outcome: Option<String>,
    pub units: Vec<Unit>,
    pub log: Vec<LogEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatPreview {
    pub damage: i32,
    pub retaliation: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupplySource {
    Depot,
    Truck(String),
}

fn result(ok: bool, message: impl Into<String>) -> ActionResult {
    ActionResult {
        ok,
        message: message.into(),
    }
}

pub fn position(hex: Hex) -> (f64, f64) {
    (
        f64::from(44 + hex.c * 72 + (hex.r % 2) * 36),
        42. + f64::from(hex.r) * 62.35,
    )
}

pub fn distance(a: Hex, b: Hex) -> i32 {
    let aq = a.c - (a.r - (a.r & 1)) / 2;
    let bq = b.c - (b.r - (b.r & 1)) / 2;
    ((aq - bq).abs() + (a.r - b.r).abs() + (aq + a.r - bq - b.r).abs()) / 2
}

pub fn terrain(hex: Hex) -> &'static str {
    let Hex { c, r } = hex;
    if !(0..COLS).contains(&c) || !(0..ROWS).contains(&r) {
        return "sea";
    }
    let (x, y) = position(hex);
    let i = COAST.iter().position(|&(cx, _)| cx >= x).unwrap();
    let (x1, y1) = COAST[i - 1];
    let (x2, y2) = COAST[i];
    let coastline = y1 + (y2 - y1) * ((x - x1) / (x2 - x1));
    if y < coastline + 8. {
        return "sea";
    }
    if (r == 6 && (2..=6).contains(&c)) || (r == 8 && (10..=12).contains(&c)) {
        "ridge"
    } else {
        "desert"
    }
}

/// Row-major order is also the deterministic AI tie-break order.
pub fn all_hexes() -> Vec<Hex> {
    (0..COLS * ROWS)
        .map(|i| Hex {
            c: i % COLS,
            r: i / COLS,
        })
        .collect()
}

pub fn hexes() -> Vec<Hex> {
    all_hexes()
        .into_iter()
        .filter(|&h| terrain(h) != "sea")
        .collect()
}

pub fn passable(role: &Role, hex: Hex) -> bool {
    (0..COLS).contains(&hex.c)
        && (0..ROWS).contains(&hex.r)
        && match role.domain {
            Domain::Land => terrain(hex) != "sea",
            Domain::Sea => terrain(hex) == "sea",
            Domain::Air => true,
        }
}

pub fn neighbors(hex: Hex) -> Vec<Hex> {
    hexes()
        .into_iter()
        .filter(|&h| distance(h, hex) == 1)
        .collect()
}

pub fn unit_at(state: &State, hex: Hex) -> Option<&Unit> {
    state.units.iter().find(|u| u.hp > 0 && u.hex() == hex)
}

fn make_unit(id: &str, name: &str, short: &str, side: &str, unit_type: &str, hex: Hex) -> Unit {
    let cp = role(if unit_type == "tank" {
        "crusader"
    } else {
        unit_type
    })
    .unwrap()
    .cp;
    Unit {
        id: id.into(),
        identity_id: None,
        name: name.into(),
        short: short.into(),
        side: side.into(),
        unit_type: unit_type.into(),
        c: hex.c,
        r: hex.r,
        hp: 100,
        cp,
        max_cp: cp,
        fuel: 100,
        water: 100,
        ammo: 100,
        entrenched: false,
    }
}

pub fn new_game() -> State {
    State {
        stage: 1,
        outcome: None,
        units: vec![
            make_unit(
                "7arm",
                "7th Armoured Brigade",
                "7 AB",
                "allied",
                "tank",
                Hex { c: 8, r: 5 },
            ),
            make_unit(
                "4arm",
                "4th Armoured Brigade",
                "4 AB",
                "allied",
                "tank",
                Hex { c: 9, r: 7 },
            ),
            make_unit(
                "22inf",
                "22nd Guards Brigade",
                "22 GD",
                "allied",
                "infantry",
                Hex { c: 10, r: 6 },
            ),
            make_unit(
                "supply",
                "Royal Army Service Corps",
                "RASC",
                "allied",
                "truck",
                Hex { c: 10, r: 8 },
            ),
            make_unit(
                "15pz",
                "15th Panzer Division",
                "15 PZ",
                "axis",
                "tank",
                Hex { c: 6, r: 5 },
            ),
            make_unit(
                "21pz",
                "21st Panzer Division",
                "21 PZ",
                "axis",
                "tank",
                Hex { c: 5, r: 7 },
            ),
            make_unit(
                "garrison",
                "Tobruk Garrison",
                "GAR",
                "axis",
                "infantry",
                TOBRUK,
            ),
        ],
        log: vec![
            LogEntry {
                stage: 1,
                text: "Reconnaissance reports Axis armour west of the frontier.".into(),
                log_type: "info".into(),
            },
            LogEntry {
                stage: 1,
                text: "Orders received. Advance west and secure Tobruk.".into(),
                log_type: "order".into(),
            },
        ],
    }
}

fn log(state: &mut State, text: impl Into<String>, log_type: &str) {
    state.log.insert(
        0,
        LogEntry {
            stage: state.stage,
            text: text.into(),
            log_type: log_type.into(),
        },
    );
    state.log.truncate(40);
}

fn ready(state: &State, unit: Option<&Unit>, side: &str) -> bool {
    state.outcome.is_none() && unit.is_some_and(|u| u.hp > 0 && u.side == side)
}

/// Place a sandbox reinforcement, not an imported historical counter's stats.
pub fn deploy_unit(state: &mut State, role_id: &str, side: &str, hex: Hex) -> ActionResult {
    let Some(role) = role(role_id) else {
        return result(false, "Unknown sandbox role.");
    };
    if state.outcome.is_some()
        || !matches!(side, "allied" | "axis")
        || !passable(role, hex)
        || unit_at(state, hex).is_some()
    {
        return result(
            false,
            "Choose an empty hex in this unit's movement domain during an active operation.",
        );
    }
    let id = format!("reinforcement-{}", state.units.len() + 1);
    let short = format!("R{}", state.units.len() + 1);
    state
        .units
        .push(make_unit(&id, role.name, &short, side, role_id, hex));
    log(
        state,
        format!(
            "{} deployed as a fictional sandbox reinforcement.",
            role.name
        ),
        "order",
    );
    result(
        true,
        format!(
            "{} deployed. Sandbox values, not historical characteristics.",
            role.name
        ),
    )
}

/// Dijkstra uses the same terrain costs for highlights and executed movement.
pub fn reachable(state: &State, unit: Option<&Unit>) -> HashMap<Hex, i32> {
    let mut costs = HashMap::new();
    let Some(unit) = unit else {
        return costs;
    };
    if unit.hp <= 0 || state.outcome.is_some() || unit.water <= 0 {
        return costs;
    }
    let budget = unit.cp.min(if unit.role().motorized {
        unit.fuel / 8
    } else {
        i32::MAX
    });
    costs.insert(unit.hex(), 0);
    let mut queue = vec![(unit.hex(), 0)];
    while !queue.is_empty() {
        queue.sort_by_key(|&(_, cost)| cost);
        let (current, current_cost) = queue.remove(0);
        if current_cost > costs[&current] {
            continue;
        }
        // Entering enemy control ends movement; a later order may leave it.
        if current_cost > 0
            && unit.role().domain != Domain::Air
            && state.units.iter().any(|u| {
                u.hp > 0
                    && u.side != unit.side
                    && u.role().domain == unit.role().domain
                    && u.role().strength > 0
                    && distance(u.hex(), current) == 1
            })
        {
            continue;
        }
        for next in all_hexes()
            .into_iter()
            .filter(|&h| distance(h, current) == 1 && passable(unit.role(), h))
        {
            if unit.role().domain != Domain::Air
                && unit_at(state, next).is_some_and(|u| u.id != unit.id)
            {
                continue;
            }
            let cost = current_cost
                + if unit.role().domain == Domain::Land && terrain(next) == "ridge" {
                    2
                } else {
                    1
                };
            if cost <= budget && cost < *costs.get(&next).unwrap_or(&i32::MAX) {
                costs.insert(next, cost);
                queue.push((next, cost));
            }
        }
    }
    costs.remove(&unit.hex());
    // Aircraft may overfly occupied hexes, but this sandbox does not stack units.
    costs.retain(|hex, _| unit_at(state, *hex).is_none());
    costs
}

pub fn check_outcome(state: &mut State) -> Option<String> {
    if unit_at(state, TOBRUK).is_some_and(|u| u.side == "allied" && u.role().can_capture()) {
        state.outcome = Some("victory".into());
    } else if !state
        .units
        .iter()
        .any(|u| u.side == "allied" && u.role().can_capture() && u.hp > 0)
        || state.stage > STAGE_LIMIT
    {
        state.outcome = Some("defeat".into());
    }
    state.outcome.clone()
}

pub fn move_unit(state: &mut State, id: &str, destination: Hex) -> ActionResult {
    let index = state.units.iter().position(|u| u.id == id);
    if !ready(state, index.map(|i| &state.units[i]), "allied") {
        return result(false, "Select an active Commonwealth unit.");
    }
    let index = index.unwrap();
    let Some(&cost) = reachable(state, Some(&state.units[index])).get(&destination) else {
        return result(
            false,
            "That hex is not reachable. Choose a highlighted hex.",
        );
    };
    let unit = &mut state.units[index];
    unit.c = destination.c;
    unit.r = destination.r;
    unit.cp -= cost;
    if unit.role().motorized {
        unit.fuel -= cost * 8;
    }
    unit.entrenched = false;
    let message = format!("{} has moved. {} CP remaining.", unit.name, unit.cp);
    let entry = format!(
        "{} advanced to {}:{}. −{} capability point{}.",
        unit.short,
        unit.c + 1,
        unit.r + 1,
        cost,
        if cost == 1 { "" } else { "s" }
    );
    log(state, entry, "move");
    check_outcome(state);
    result(true, message)
}

pub fn can_attack(state: &State, attacker: Option<&Unit>, target: Option<&Unit>) -> bool {
    let (Some(a), Some(b)) = (attacker, target) else {
        return false;
    };
    state.outcome.is_none()
        && a.hp > 0
        && b.hp > 0
        && a.side != b.side
        && a.role().can_target(b.role())
        && a.cp >= 2
        && a.ammo >= 20
        && a.water > 0
        && (1..=a.role().range).contains(&distance(a.hex(), b.hex()))
}

pub fn combat_preview(attacker: &Unit, target: &Unit) -> CombatPreview {
    let cover = if target.role().domain == Domain::Land && terrain(target.hex()) == "ridge" {
        8
    } else {
        0
    } + if target.entrenched { 10 } else { 0 };
    let strength = attacker.role().strength
        + if attacker.role().id == "anti-tank"
            && matches!(
                target.role().id,
                "crusader" | "panzer" | "italian-tank" | "armored-car"
            )
        {
            10
        } else {
            0
        };
    let damage = (8).max((f64::from(strength * attacker.hp) / 100.).round() as i32 - cover);
    let retaliation = if !target.role().can_target(attacker.role())
        || distance(attacker.hex(), target.hex()) > target.role().range
        || target.ammo < 10
    {
        0
    } else {
        let strength = if matches!(target.role().id, "crusader" | "panzer") {
            18
        } else {
            (target.role().strength * 4 / 9).max(1)
        };
        (4).max(
            (f64::from(strength * target.hp) / 100. * if target.water > 0 { 1. } else { 0.5 })
                .round() as i32,
        )
    };
    CombatPreview {
        damage,
        retaliation,
    }
}

pub fn attack(state: &mut State, attacker_id: &str, target_id: &str, side: &str) -> ActionResult {
    let ai = state.units.iter().position(|u| u.id == attacker_id);
    let bi = state.units.iter().position(|u| u.id == target_id);
    if !ready(state, ai.map(|i| &state.units[i]), side)
        || !can_attack(
            state,
            ai.map(|i| &state.units[i]),
            bi.map(|i| &state.units[i]),
        )
    {
        return result(
            false,
            "Attack needs a target in this role's range and domain, 2 CP, water, and 20 ammunition.",
        );
    }
    let (ai, bi) = (ai.unwrap(), bi.unwrap());
    let CombatPreview {
        damage,
        retaliation,
    } = combat_preview(&state.units[ai], &state.units[bi]);
    state.units[ai].cp -= 2;
    state.units[ai].ammo -= 20;
    state.units[ai].entrenched = false;
    state.units[bi].hp = (state.units[bi].hp - damage).max(0);
    if state.units[bi].hp > 0 {
        state.units[ai].hp = (state.units[ai].hp - retaliation).max(0);
        if retaliation > 0 {
            state.units[bi].ammo = (state.units[bi].ammo - 10).max(0);
        }
    }
    let suffix = if state.units[bi].hp <= 0 {
        ". Enemy formation destroyed.".into()
    } else {
        format!(", {retaliation} returned.")
    };
    let message = format!(
        "{} assaulted {}: {damage} damage{suffix}",
        state.units[ai].short, state.units[bi].short
    );
    log(state, &message, "combat");
    check_outcome(state);
    result(true, message)
}

pub fn dig_in(state: &mut State, id: &str) -> ActionResult {
    let index = state.units.iter().position(|u| u.id == id);
    if !ready(state, index.map(|i| &state.units[i]), "allied")
        || index.is_some_and(|i| {
            let u = &state.units[i];
            u.cp < 1 || u.entrenched || !u.role().can_capture()
        })
    {
        return result(
            false,
            "Digging in needs 1 CP and a combat unit not already entrenched.",
        );
    }
    let unit = &mut state.units[index.unwrap()];
    unit.entrenched = true;
    unit.cp -= 1;
    let message = format!(
        "{} is dug in. Moving or attacking removes cover.",
        unit.name
    );
    let entry = format!("{} dug in. Incoming damage reduced by 10.", unit.short);
    log(state, entry, "order");
    result(true, message)
}

pub fn supply_source(state: &State, unit: Option<&Unit>) -> Option<SupplySource> {
    let unit = unit?;
    if distance(unit.hex(), DEPOT) <= 2 {
        return Some(SupplySource::Depot);
    }
    state
        .units
        .iter()
        .find(|u| {
            u.side == unit.side
                && u.role().support == Support::Supply
                && u.hp > 0
                && u.id != unit.id
                && u.water >= 25
                && u.fuel >= 25
                && u.ammo >= 25
                && distance(u.hex(), unit.hex()) <= 2
        })
        .map(|u| SupplySource::Truck(u.id.clone()))
}

pub fn can_support(state: &State, source: Option<&Unit>, target: Option<&Unit>) -> bool {
    let (Some(a), Some(b)) = (source, target) else {
        return false;
    };
    ready(state, Some(a), "allied")
        && b.hp > 0
        && a.side == b.side
        && a.water > 0
        && distance(a.hex(), b.hex()) == 1
        && match a.role().support {
            Support::Command => a.cp >= 2 && b.cp < b.max_cp,
            Support::Repair => {
                a.cp >= 1
                    && a.fuel >= 20
                    && b.hp < 100
                    && b.role().motorized
                    && b.role().domain == Domain::Land
            }
            Support::Fortify => a.cp >= 1 && !b.entrenched && b.role().can_capture(),
            _ => false,
        }
}

pub fn support_unit(state: &mut State, source_id: &str, target_id: &str) -> ActionResult {
    let a = state.units.iter().position(|u| u.id == source_id);
    let b = state.units.iter().position(|u| u.id == target_id);
    if !can_support(
        state,
        a.map(|i| &state.units[i]),
        b.map(|i| &state.units[i]),
    ) {
        return result(
            false,
            "Choose an adjacent eligible friendly unit with the required resources.",
        );
    }
    let (a, b) = (a.unwrap(), b.unwrap());
    let action = match state.units[a].role().support {
        Support::Command => {
            state.units[a].cp -= 2;
            state.units[b].cp += 1;
            "restored one CP to"
        }
        Support::Repair => {
            state.units[a].cp -= 1;
            state.units[a].fuel -= 20;
            state.units[b].hp = (state.units[b].hp + 20).min(100);
            "repaired"
        }
        Support::Fortify => {
            state.units[a].cp -= 1;
            state.units[b].entrenched = true;
            "fortified"
        }
        _ => unreachable!(),
    };
    let message = format!(
        "{} {action} {}.",
        state.units[a].short, state.units[b].short
    );
    log(state, &message, "order");
    result(true, message)
}

pub fn resupply(state: &mut State, id: &str) -> ActionResult {
    let index = state.units.iter().position(|u| u.id == id);
    let source = supply_source(state, index.map(|i| &state.units[i]));
    if !ready(state, index.map(|i| &state.units[i]), "allied")
        || index.is_some_and(|i| state.units[i].cp < 1)
        || source.is_none()
    {
        return result(
            false,
            "Resupply needs 1 CP and a stocked truck or depot within 2 hexes.",
        );
    }
    let index = index.unwrap();
    let source = source.unwrap();
    let unit = &mut state.units[index];
    if unit.fuel == 100 && unit.water == 100 && unit.ammo == 100 {
        return result(false, "This formation is already fully supplied.");
    }
    let limit = if source == SupplySource::Depot {
        100
    } else {
        25
    };
    let fuel = (100 - unit.fuel).min(limit);
    let water = (100 - unit.water).min(limit);
    let ammo = (100 - unit.ammo).min(limit);
    unit.fuel += fuel;
    unit.water += water;
    unit.ammo += ammo;
    unit.cp -= 1;
    let entry = format!(
        "{} resupplied from {}.",
        unit.short,
        if source == SupplySource::Depot {
            "the frontier depot"
        } else {
            "a supply unit"
        }
    );
    if let SupplySource::Truck(id) = source {
        let truck = state.units.iter_mut().find(|u| u.id == id).unwrap();
        truck.fuel -= fuel;
        truck.water -= water;
        truck.ammo -= ammo;
    }
    log(state, entry, "supply");
    result(
        true,
        format!("Supplies received: +{fuel} fuel, +{water} water, +{ammo} ammunition."),
    )
}

pub fn end_stage(state: &mut State) -> ActionResult {
    if state.outcome.is_some() {
        return result(
            false,
            "This operation has ended. Start a new operation to play again.",
        );
    }
    let enemies: Vec<usize> = state
        .units
        .iter()
        .enumerate()
        .filter(|(_, u)| u.side == "axis" && u.hp > 0)
        .map(|(i, _)| i)
        .collect();
    for ei in enemies {
        state.units[ei].cp = state.units[ei].max_cp;
        let enemy = &state.units[ei];
        let target = state
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.side == "allied" && u.hp > 0 && enemy.role().can_target(u.role()))
            .min_by_key(|(_, u)| distance(enemy.hex(), u.hex()))
            .map(|(i, _)| i);
        let Some(ti) = target else {
            continue;
        };
        let target_hex = state.units[ti].hex();
        if enemy.id != "garrison" && distance(enemy.hex(), target_hex) > enemy.role().range {
            let moves = reachable(state, Some(enemy));
            let best = all_hexes()
                .into_iter()
                .filter(|h| moves.get(h).is_some_and(|&cost| cost <= 2))
                .min_by_key(|&h| distance(h, target_hex));
            if let Some(best) =
                best.filter(|&h| distance(h, target_hex) < distance(enemy.hex(), target_hex))
            {
                let cost = moves[&best];
                let enemy = &mut state.units[ei];
                enemy.c = best.c;
                enemy.r = best.r;
                enemy.cp -= cost;
                if enemy.role().motorized {
                    enemy.fuel -= cost * 8;
                }
                enemy.entrenched = false;
                let entry = format!("{} repositioned toward Commonwealth forces.", enemy.short);
                log(state, entry, "move");
            }
        }
        if can_attack(state, Some(&state.units[ei]), Some(&state.units[ti])) {
            let attacker_id = state.units[ei].id.clone();
            let target_id = state.units[ti].id.clone();
            attack(state, &attacker_id, &target_id, "axis");
        }
        if state.outcome.is_some() {
            return result(true, "The last combat formation has been lost.");
        }
    }
    state.stage += 1;
    let hot = state.stage % 3 == 0;
    for unit in state.units.iter_mut().filter(|u| u.hp > 0) {
        unit.water = (unit.water - if hot { 18 } else { 10 }).max(0);
        if unit.role().motorized {
            unit.fuel = (unit.fuel - if hot { 5 } else { 2 }).max(0);
        }
        if unit.water == 0 {
            unit.hp = (unit.hp - 10).max(0);
        }
        unit.cp = unit.max_cp;
    }
    log(
        state,
        if hot {
            "Hot winds. Water consumption increased to 18; vehicle fuel loss to 5."
        } else {
            "Supply upkeep: −10 water and −2 vehicle fuel per formation."
        },
        "supply",
    );
    check_outcome(state);
    result(
        true,
        if state.outcome.is_some() {
            "The operation has ended.".into()
        } else {
            format!(
                "Operations stage {}. Commonwealth orders are open.",
                state.stage
            )
        },
    )
}

pub fn date_label(stage: i32) -> String {
    // Gregorian civil date from days since 1970-01-01; 1941-11-18 is day -10271.
    let days = -10271_i64 + ((i64::from(stage) - 1) * 7).div_euclid(3);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{day} {} {year}", names[(month - 1) as usize])
}
