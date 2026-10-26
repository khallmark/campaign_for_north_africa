//! Local, open-information rulebook play. Orders are player adjudications, not
//! certified movement/combat. No sandbox statistics or historical setup defaults.
//! Saves contain the starting position and orders; loading replays every order.
use crate::official::{
    fuel,
    rng::RecordedRng,
    state::{Calendar, FuelPoints, RngState, Side, Supplies},
};
use cna_map::{geometry, hex::HexId};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, num::NonZeroU32};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Piece {
    pub id: usize,
    pub identity: String,
    pub side: Side,
    pub hex: HexId,
    pub supplies: Option<Supplies>,
    pub eliminated: bool,
    pub dug_in: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub name: String,
    pub calendar: Calendar,
    pub first: Side,
    pub pieces: Vec<Piece>,
    pub seed: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Order {
    Move {
        id: usize,
        to: HexId,
        fuel: Option<FuelPoints>,
        note: String,
    },
    Combat {
        attacker: usize,
        defender: usize,
        attacker_lost: bool,
        defender_lost: bool,
        note: String,
    },
    Supplies {
        id: usize,
        supplies: Supplies,
        note: String,
    },
    DigIn {
        id: usize,
        note: String,
    },
    Roll {},
    EndTurn {
        note: String,
    },
    /// Initiative and all pre-operation phases are adjudicated off-board.
    BeginStage {
        first: Side,
        note: String,
    },
    Concede {},
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    schema: u8,
    setup: Setup,
    orders: Vec<Order>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    save: Save,
    pub pieces: Vec<Piece>,
    pub calendar: Calendar,
    pub active: Side,
    first: Side,
    pub awaiting_stage: bool,
    pub winner: Option<Side>,
    pub log: Vec<String>,
    rng: RngState,
}

pub fn side_name(side: Side) -> &'static str {
    match side {
        Side::Axis => "Axis",
        Side::Commonwealth => "Commonwealth",
    }
}

fn other(side: Side) -> Side {
    match side {
        Side::Axis => Side::Commonwealth,
        Side::Commonwealth => Side::Axis,
    }
}

fn note(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > 2000 {
        Err("Record the rulebook adjudication (1–2000 characters).")
    } else {
        Ok(())
    }
}

impl Game {
    pub fn new(setup: Setup) -> Result<Self, &'static str> {
        note(&setup.name)?;
        let mut ids = BTreeSet::new();
        if setup.pieces.len() > 2000
            || setup.pieces.iter().any(|p| {
                !ids.insert(p.id)
                    || p.identity.trim().is_empty()
                    || p.identity.len() > 200
                    || !geometry::contains_hex(p.hex)
                    || p.eliminated
            })
        {
            return Err("Setup requires distinct, identified counters inside mapped sections.");
        }
        if ![Side::Axis, Side::Commonwealth]
            .iter()
            .all(|side| setup.pieces.iter().any(|p| p.side == *side))
        {
            return Err("Place at least one Axis and one Commonwealth counter before starting.");
        }
        let mut seed = [0; 32];
        seed[..4].copy_from_slice(&setup.seed.to_le_bytes());
        Ok(Self {
            pieces: setup.pieces.clone(),
            calendar: setup.calendar,
            active: setup.first,
            first: setup.first,
            awaiting_stage: false,
            winner: None,
            log: vec![format!(
                "{} — rulebook hot-seat; player deployment, no automatic scenario certification.",
                setup.name
            )],
            rng: RngState {
                seed,
                word_position: 0,
            },
            save: Save {
                schema: 1,
                setup,
                orders: Vec::new(),
            },
        })
    }

    pub fn name(&self) -> &str {
        &self.save.setup.name
    }

    /// Validate on a working copy: a rejected order changes neither state nor dice.
    pub fn apply(&mut self, order: Order) -> Result<(), &'static str> {
        if self.save.orders.len() >= 10000 {
            return Err("Session order limit reached; export this game.");
        }
        let mut next = self.clone();
        next.record(order)?;
        if next.save().len() > 4_000_000 {
            return Err("Session save limit reached; export this game.");
        }
        *self = next;
        Ok(())
    }

    fn record(&mut self, order: Order) -> Result<(), &'static str> {
        let prefix = format!(
            "Turn {} / Stage {} · {}",
            self.calendar.game_turn(),
            self.calendar.operations_stage(),
            side_name(self.active)
        );
        let message = self.execute(&order)?;
        self.log.push(format!("{prefix}: {message}"));
        self.save.orders.push(order);
        Ok(())
    }

    fn own(&mut self, id: usize) -> Result<&mut Piece, &'static str> {
        self.pieces
            .iter_mut()
            .find(|p| p.id == id && p.side == self.active && !p.eliminated)
            .ok_or("Select a surviving counter belonging to the active player.")
    }

    fn execute(&mut self, order: &Order) -> Result<String, &'static str> {
        if self.winner.is_some() {
            return Err("This game has ended by concession.");
        }
        if self.awaiting_stage && !matches!(order, Order::BeginStage { .. } | Order::Concede {}) {
            return Err(
                "Resolve initiative, weather and the other rulebook phases before the next stage.",
            );
        }
        match order {
            Order::Move {
                id,
                to,
                fuel: cost,
                note: reason,
            } => {
                note(reason)?;
                if !geometry::contains_hex(*to) {
                    return Err("Destination is outside mapped section geometry.");
                }
                let piece = self.own(*id)?;
                if piece.hex == *to {
                    return Err("Choose a different destination.");
                }
                if let Some(cost) = cost {
                    let supplies = piece
                        .supplies
                        .as_mut()
                        .ok_or("Record starting supplies before debiting fuel.")?;
                    fuel::draw(&mut supplies.fuel, *cost)
                        .map_err(|_| "Insufficient fuel for this adjudicated move.")?;
                }
                let from = piece.hex;
                piece.hex = *to;
                piece.dug_in = false;
                Ok(format!(
                    "Counter {id}: {from} → {to}. Player-adjudicated route/cost: {reason}"
                ))
            }
            Order::Combat {
                attacker,
                defender,
                attacker_lost,
                defender_lost,
                note: reason,
            } => {
                note(reason)?;
                self.own(*attacker)?;
                let target = self
                    .pieces
                    .iter_mut()
                    .find(|p| p.id == *defender && p.side != self.active && !p.eliminated)
                    .ok_or("Choose a surviving enemy counter.")?;
                target.eliminated = *defender_lost;
                self.own(*attacker)?.eliminated = *attacker_lost;
                Ok(format!("Combat {attacker} → {defender}: attacker eliminated={attacker_lost}, defender eliminated={defender_lost}. Player-adjudicated result: {reason}"))
            }
            Order::Supplies {
                id,
                supplies,
                note: reason,
            } => {
                note(reason)?;
                self.own(*id)?.supplies = Some(*supplies);
                Ok(format!(
                    "Counter {id}: supply ledger updated by player adjudication: {reason}"
                ))
            }
            Order::DigIn { id, note: reason } => {
                note(reason)?;
                self.own(*id)?.dug_in = true;
                Ok(format!(
                    "Counter {id}: construction recorded; no automatic combat modifier. {reason}"
                ))
            }
            Order::Roll {} => {
                let mut rng = RecordedRng::new(&self.rng);
                let roll = rng
                    .bounded(NonZeroU32::new(6).unwrap())
                    .map_err(|_| "Dice stream exhausted.")?
                    + 1;
                self.rng = rng.state().clone();
                Ok(format!(
                    "D6 = {roll} (recorded ChaCha8 roll; no automatic combat resolution)."
                ))
            }
            Order::EndTurn { note: reason } => {
                note(reason)?;
                if self.active == self.first {
                    self.active = other(self.active);
                } else {
                    self.awaiting_stage = true;
                }
                Ok(format!(
                    "Player portion completed; phases adjudicated: {reason}"
                ))
            }
            Order::BeginStage {
                first,
                note: reason,
            } => {
                note(reason)?;
                if !self.awaiting_stage {
                    return Err("Both players must finish before advancing the stage.");
                }
                self.calendar = self
                    .calendar
                    .next_operations_stage()
                    .ok_or("The campaign calendar has ended; record the result by concession.")?;
                self.first = *first;
                self.active = *first;
                self.awaiting_stage = false;
                Ok(format!("Next stage begins, {} first. Player-adjudicated initiative and preliminary phases: {reason}", side_name(*first)))
            }
            Order::Concede {} => {
                self.winner = Some(other(self.active));
                Ok(format!(
                    "Conceded. {} wins by concession, not an automated scenario victory check.",
                    side_name(other(self.active))
                ))
            }
        }
    }

    pub fn save(&self) -> String {
        serde_json::to_string(&self.save).expect("serializable session")
    }

    pub fn load(json: &str) -> Result<Self, &'static str> {
        if json.len() > 4_000_000 {
            return Err("Save exceeds 4 MB.");
        }
        let save: Save =
            serde_json::from_str(json).map_err(|_| "Invalid rulebook session save.")?;
        if save.schema != 1 || save.orders.len() > 10000 {
            return Err("Unsupported save version or order count.");
        }
        let mut game = Self::new(save.setup)?;
        for order in save.orders {
            // This position is not installed until the entire replay succeeds.
            game.record(order)?;
        }
        Ok(game)
    }
}
