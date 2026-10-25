//! Atomic command execution. Kernels are trusted Rust functions, not client input.
use super::{
    rng::RecordedRng,
    state::{GameState, RngState, Side},
};
use crate::contract::command::{Command, Event, Outcome};
use cna_catalog::CanonicalUnitId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Deliberately carries no hidden-state or random-draw diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unavailable;

impl Unavailable {
    pub fn outcome<E>(self) -> Outcome<E> {
        Outcome::Rejected {
            redacted_reason: "command unavailable".into(),
        }
    }
}

#[derive(Clone, Serialize)]
struct Receipt<P, E> {
    command: Command<P>,
    outcome: Outcome<E>,
}

/// Authoritative in-memory session, NOT a viewer projection or a save format.
/// Its serialized diagnostic snapshot contains private state and receipts.
/// Restore/replay validation is a separate boundary; do not recreate a running
/// session from GameState alone, which would discard its idempotency history.
#[derive(Clone, Serialize)]
pub struct Session<P, E> {
    game: GameState,
    actors: BTreeMap<String, Side>,
    catalog_ids: BTreeSet<CanonicalUnitId>,
    events: Vec<Event<E>>,
    next_sequence: u64,
    receipts: BTreeMap<String, Receipt<P, E>>,
}

impl<P: Clone + Eq, E: Clone> Session<P, E> {
    /// Start a new event history from a trusted, validated initial position.
    /// Caller verifies the reference contents and supplies authorized actors.
    pub fn new(
        game: GameState,
        actors: BTreeMap<String, Side>,
        catalog_ids: BTreeSet<CanonicalUnitId>,
    ) -> Result<Self, String> {
        game.validate(&catalog_ids)?;
        if actors.keys().any(|id| id.trim().is_empty()) {
            return Err("empty actor identifier".into());
        }
        Ok(Self {
            game,
            actors,
            catalog_ids,
            events: Vec::new(),
            next_sequence: 0,
            receipts: BTreeMap::new(),
        })
    }

    pub fn state(&self) -> &GameState {
        &self.game
    }
    pub fn events(&self) -> &[Event<E>] {
        &self.events
    }

    /// The host supplies an authenticated principal independently of the command.
    /// Kernel selection is bound to the loaded ruleset by the host, never by a
    /// client-provided callback. Kernels must have no effects outside `tx` and
    /// payload equality must compare every field, including optional annotations.
    pub fn reduce(
        &mut self,
        authenticated_actor: &str,
        command: &Command<P>,
        execute: impl FnOnce(Side, &P, &mut Transaction<E>) -> Result<(), Unavailable>,
    ) -> Outcome<E> {
        // Authorize before looking up a receipt: cached results are private too.
        let Some(&side) = self.actors.get(authenticated_actor) else {
            return Unavailable.outcome();
        };
        if command.actor != authenticated_actor || command.command_id.trim().is_empty() {
            return Unavailable.outcome();
        }
        if let Some(receipt) = self.receipts.get(&command.command_id) {
            return if receipt.command == *command {
                receipt.outcome.clone()
            } else {
                Unavailable.outcome()
            };
        }
        if command.ruleset_id != self.game.references.ruleset_id
            || command.ruleset_digest != self.game.references.ruleset
            || command.game_revision != self.game.revision
        {
            return Unavailable.outcome();
        }
        let Some(new_revision) = self.game.revision.checked_add(1) else {
            return Unavailable.outcome();
        };
        let mut tx = Transaction {
            game: self.game.clone(),
            rng: self.game.rng.clone(),
            command_id: command.command_id.clone(),
            next_sequence: self.next_sequence,
            events: Vec::new(),
            failed: false,
        };
        if execute(side, &command.payload, &mut tx).is_err()
            || tx.failed
            || tx.game.revision != self.game.revision
            || tx.game.rng != self.game.rng
            || tx.game.references != self.game.references
            || tx.game.validate(&self.catalog_ids).is_err()
        {
            return Unavailable.outcome();
        }
        tx.game.revision = new_revision;
        tx.game.rng = tx.rng;
        let outcome = Outcome::Accepted {
            events: tx.events.clone(),
            new_revision,
        };
        self.game = tx.game;
        self.next_sequence = tx.next_sequence;
        self.events.extend(tx.events);
        self.receipts.insert(
            command.command_id.clone(),
            Receipt {
                command: command.clone(),
                outcome: outcome.clone(),
            },
        );
        outcome
    }
}

/// Private working copy. Game-specific state may change; revision, reference
/// digests and RNG are reducer-owned and direct edits to those fields reject.
pub struct Transaction<E> {
    pub game: GameState,
    rng: RngState,
    command_id: String,
    next_sequence: u64,
    events: Vec<Event<E>>,
    failed: bool,
}

impl<E> Transaction<E> {
    /// Compute one ordered event, attaching every random word consumed by it.
    /// A failed event poisons the transaction even if a kernel ignores its error.
    pub fn emit(
        &mut self,
        produce: impl FnOnce(&mut RecordedRng) -> Result<E, Unavailable>,
    ) -> Result<(), Unavailable> {
        if self.failed {
            return Err(Unavailable);
        }
        let Some(next) = self.next_sequence.checked_add(1) else {
            self.failed = true;
            return Err(Unavailable);
        };
        let mut rng = RecordedRng::new(&self.rng);
        match produce(&mut rng) {
            Ok(payload) if !rng.failed => {
                let (state, random_draws) = rng.into_parts();
                self.events.push(Event {
                    sequence: self.next_sequence,
                    command_id: self.command_id.clone(),
                    payload,
                    random_draws,
                });
                self.rng = state;
                self.next_sequence = next;
                Ok(())
            }
            _ => {
                self.failed = true;
                Err(Unavailable)
            }
        }
    }
}

/// Diagnostic kernel payload/event. No game rule executes in this kernel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Noop {}

pub fn noop(_: Side, _: &Noop, tx: &mut Transaction<Noop>) -> Result<(), Unavailable> {
    tx.emit(|_| Ok(Noop {}))
}
