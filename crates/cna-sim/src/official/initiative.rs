//! L21 §7 (p14), original §7.2 chart (R79 physical p97).
//! Host-supplied roles and opening override are trusted scenario references,
//! never command fields. This kernel does not advance the phase/calendar.
use super::{
    reducer::{Transaction, Unavailable},
    state::{Calendar, GameState, Side},
};
use cna_catalog::CanonicalUnitId;
use cna_map::hex::{MapLocation, OffMapBox};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap, num::NonZeroU32};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitiativeState {
    pub determined_at: Calendar,
    pub winner: Side,
    pub first_players: [Option<Side>; 3],
}

impl InitiativeState {
    pub fn first_player(&self, at: Calendar) -> Option<Side> {
        (self.determined_at.game_turn() == at.game_turn())
            .then(|| self.first_players[usize::from(at.operations_stage() - 1)])
            .flatten()
    }

    pub(crate) fn valid_at(&self, at: Calendar) -> bool {
        let determined = (
            self.determined_at.game_turn(),
            self.determined_at.operations_stage(),
        );
        if determined > (at.game_turn(), at.operations_stage()) {
            return false;
        }
        self.first_players.iter().enumerate().all(|(i, choice)| {
            choice.is_none()
                || (i + 1 >= usize::from(self.determined_at.operations_stage())
                    && (self.determined_at.game_turn() < at.game_turn()
                        || i < usize::from(at.operations_stage())))
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitiativeRole {
    Rommel,
    GermanLandCombat,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub struct OpeningInitiative {
    pub at: Calendar,
    pub winner: Side,
}

/// Bound by the host to the loaded reference digests. Unknown identities and
/// ambiguous VASSAL port holding boxes reject instead of inventing presence.
pub struct InitiativeRules {
    pub roles: BTreeMap<CanonicalUnitId, InitiativeRole>,
    pub opening: Option<OpeningInitiative>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InitiativeRatings {
    pub axis: u8,
    pub commonwealth: u8,
}

impl InitiativeRatings {
    pub fn winner(self, axis_die: u8, commonwealth_die: u8) -> Result<Option<Side>, Unavailable> {
        if !(1..=6).contains(&axis_die) || !(1..=6).contains(&commonwealth_die) {
            return Err(Unavailable);
        }
        Ok(
            match (u16::from(self.axis) + u16::from(axis_die))
                .cmp(&(u16::from(self.commonwealth) + u16::from(commonwealth_die)))
            {
                Ordering::Greater => Some(Side::Axis),
                Ordering::Less => Some(Side::Commonwealth),
                Ordering::Equal => None,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InitiativeCommand {
    Determine {},
    ChooseOrder { first: bool },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InitiativeEvent {
    Rolled {
        game_turn: u16,
        axis_rating: u8,
        commonwealth_rating: u8,
        axis_die: u8,
        commonwealth_die: u8,
        winner: Option<Side>,
    },
    ScenarioInitiative {
        at: Calendar,
        winner: Side,
    },
    OrderDeclared {
        at: Calendar,
        player_a: Side,
        player_b: Side,
    },
}

impl InitiativeRules {
    pub fn ratings(&self, game: &GameState) -> Result<InitiativeRatings, Unavailable> {
        let mut axis = 1;
        for unit in game.instances.values().filter(|u| u.side == Side::Axis) {
            // These are outside the game-map for §7.2. Port display holding boxes
            // need a reviewed physical-location bridge; they are not free aliases.
            if let MapLocation::OffMap(
                OffMapBox::Tripoli
                | OffMapBox::Tripolitania
                | OffMapBox::Tunis
                | OffMapBox::Gabes
                | OffMapBox::Sicily
                | OffMapBox::Italy
                | OffMapBox::Crete
                | OffMapBox::GermanShipping,
            ) = unit.location
            {
                continue;
            }
            let role = self.roles.get(&unit.unit_id).ok_or(Unavailable)?;
            if *role == InitiativeRole::Other {
                continue;
            }
            if !matches!(unit.location, MapLocation::Hex(_)) {
                return Err(Unavailable);
            }
            axis = axis.max(match role {
                InitiativeRole::Rommel => 6,
                InitiativeRole::GermanLandCombat => 3,
                InitiativeRole::Other => unreachable!(),
            });
        }
        Ok(InitiativeRatings {
            axis,
            commonwealth: match game.calendar.game_turn() {
                1..=42 => 3,
                43..=90 => 4,
                _ => 5,
            },
        })
    }

    pub fn execute(
        &self,
        side: Side,
        command: &InitiativeCommand,
        tx: &mut Transaction<InitiativeEvent>,
    ) -> Result<(), Unavailable> {
        match command {
            InitiativeCommand::Determine {} => self.determine(tx),
            InitiativeCommand::ChooseOrder { first } => {
                let at = tx.game.calendar;
                let initiative = tx.game.initiative.as_mut().ok_or(Unavailable)?;
                let index = usize::from(at.operations_stage() - 1);
                if initiative.determined_at.game_turn() != at.game_turn()
                    || initiative.winner != side
                    || initiative.first_players[index].is_some()
                {
                    return Err(Unavailable);
                }
                let other = match side {
                    Side::Axis => Side::Commonwealth,
                    Side::Commonwealth => Side::Axis,
                };
                let (player_a, player_b) = if *first { (side, other) } else { (other, side) };
                initiative.first_players[index] = Some(player_a);
                tx.emit(|_| {
                    Ok(InitiativeEvent::OrderDeclared {
                        at,
                        player_a,
                        player_b,
                    })
                })
            }
        }
    }

    fn determine(&self, tx: &mut Transaction<InitiativeEvent>) -> Result<(), Unavailable> {
        let at = tx.game.calendar;
        if tx
            .game
            .initiative
            .as_ref()
            .is_some_and(|i| i.determined_at.game_turn() == at.game_turn())
        {
            return Err(Unavailable);
        }
        if self.opening.is_some_and(|o| {
            (at.game_turn(), at.operations_stage()) < (o.at.game_turn(), o.at.operations_stage())
        }) {
            return Err(Unavailable);
        }
        let winner = if let Some(opening) = self.opening.filter(|o| o.at == at) {
            tx.emit(|_| {
                Ok(InitiativeEvent::ScenarioInitiative {
                    at,
                    winner: opening.winner,
                })
            })?;
            opening.winner
        } else {
            if at.operations_stage() != 1 {
                return Err(Unavailable);
            }
            let ratings = self.ratings(&tx.game)?;
            loop {
                let mut winner = None;
                tx.emit(|rng| {
                    // Stable stream order is Axis then Commonwealth, for every reroll.
                    let axis_die = rng.bounded(NonZeroU32::new(6).unwrap())? as u8 + 1;
                    let commonwealth_die = rng.bounded(NonZeroU32::new(6).unwrap())? as u8 + 1;
                    winner = ratings.winner(axis_die, commonwealth_die)?;
                    Ok(InitiativeEvent::Rolled {
                        game_turn: at.game_turn(),
                        axis_rating: ratings.axis,
                        commonwealth_rating: ratings.commonwealth,
                        axis_die,
                        commonwealth_die,
                        winner,
                    })
                })?;
                if let Some(winner) = winner {
                    break winner;
                }
            }
        };
        tx.game.initiative = Some(InitiativeState {
            determined_at: at,
            winner,
            first_players: [None; 3],
        });
        Ok(())
    }
}
