//! Rulebook hot-seat bridge. The authoritative position lives in `tabletop::Game`;
//! Board counters are its render projection, never a second editable game state.
use super::{Board, Counter};
use crate::{document, view::escape};
use cna_sim::{
    contract::route::Points,
    official::state::{
        AmmunitionPoints, Calendar, FuelPoints, Side, StoresPoints, Supplies, WaterPoints,
    },
    tabletop::{side_name, Game, Order, Piece, Setup},
};
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

const SAVE_KEY: &str = "cna-rulebook-session-v1";
const LIMITS: &str = "Local, open-information hot-seat. Players adjudicate terrain, CP, reaction, combat, supply access and all rulebook phases. This is not the certified official simulation. No historical setup or ratings are invented. Do not put passwords or private save credentials in notes.";

fn input(id: &str) -> String {
    document()
        .get_element_by_id(id)
        .unwrap()
        .unchecked_into::<HtmlInputElement>()
        .value()
}
fn text(id: &str) -> String {
    document()
        .get_element_by_id(id)
        .unwrap()
        .unchecked_into::<HtmlTextAreaElement>()
        .value()
}
fn first() -> Side {
    match document()
        .get_element_by_id("play-first")
        .unwrap()
        .unchecked_into::<HtmlSelectElement>()
        .value()
        .as_str()
    {
        "axis" => Side::Axis,
        _ => Side::Commonwealth,
    }
}
fn points(value: &str) -> Result<Points, &'static str> {
    let (n, d) = value.trim().split_once('/').unwrap_or((value.trim(), "1"));
    let error = "Use nonnegative whole points or a fraction, such as 3/2.";
    Ok(Points {
        numerator: n.parse().map_err(|_| error)?,
        denominator: d.parse().map_err(|_| error)?,
    })
}
fn quantity(p: Points) -> String {
    if p.denominator.get() == 1 {
        p.numerator.to_string()
    } else {
        format!("{}/{}", p.numerator, p.denominator)
    }
}
fn field(id: &str, label: &str, value: &str) -> String {
    format!(
        r#"<label>{label}<input id="{id}" value="{}" /></label>"#,
        escape(value)
    )
}
fn first_field() -> &'static str {
    r#"<label>First player (adjudicated initiative)<select id="play-first"><option value="axis">Axis</option><option value="commonwealth">Commonwealth</option></select></label>"#
}
fn note_field() -> &'static str {
    r#"<label>Rulebook adjudication and costs<textarea id="play-note" maxlength="2000" placeholder="Record the rule reference, costs, reactions and agreed result. Required."></textarea></label>"#
}
fn submit(action: &str, label: &str) -> String {
    format!(
        r#"<p id="play-error" role="alert"></p><div class="dialog-actions"><button data-action="close">Cancel</button><button data-action="{action}" class="primary">{label}</button></div>"#
    )
}

pub(super) fn status(game: &Game) -> String {
    if let Some(winner) = game.winner {
        format!("{} wins by concession", side_name(winner))
    } else if game.awaiting_stage {
        "Stage complete · resolve next-stage phases".into()
    } else {
        format!("{} to act · rulebook hot-seat", side_name(game.active))
    }
}

impl Board {
    pub(super) fn play_piece(&self, id: usize) -> Option<&Piece> {
        self.game.as_ref()?.pieces.iter().find(|p| p.id == id)
    }

    pub(super) fn can_order(&self, id: usize) -> bool {
        self.game.as_ref().is_some_and(|game| {
            game.winner.is_none()
                && !game.awaiting_stage
                && game
                    .pieces
                    .iter()
                    .any(|p| p.id == id && p.side == game.active && !p.eliminated)
        })
    }

    fn project_game(&mut self) {
        let game = self.game.as_ref().unwrap();
        self.counters = game
            .pieces
            .iter()
            .map(|p| Counter {
                id: p.id,
                identity: p.identity.clone(),
                hex: (!p.eliminated).then_some(p.hex),
            })
            .collect();
        self.turn = usize::from(game.calendar.game_turn());
        self.stage = usize::from(game.calendar.operations_stage());
        self.placing = false;
        self.message = game.log.last().unwrap().clone();
    }

    fn save_game(&mut self) {
        let save = self.game.as_ref().unwrap().save();
        let saved = web_sys::window()
            .unwrap()
            .local_storage()
            .ok()
            .flatten()
            .is_some_and(|s| s.set_item(SAVE_KEY, &save).is_ok());
        if !saved {
            self.message.push_str(
                " · Browser storage unavailable; export from Play / Save before leaving.",
            );
        }
    }

    fn install_game(&mut self, game: Game) -> Result<(), &'static str> {
        // Save files cannot invent an artwork path, identity or side attribution.
        for piece in &game.pieces {
            let identity = self
                .inventory
                .find(&piece.identity)
                .ok_or("Save references an unknown identity.")?;
            if identity_side(&identity.side)? != piece.side {
                return Err("Save side disagrees with the identity inventory.");
            }
        }
        self.game = Some(game);
        self.selected = None;
        self.project_game();
        self.save_game();
        Ok(())
    }

    pub(super) fn restore_game(&mut self) {
        let saved = web_sys::window()
            .unwrap()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|s| s.get_item(SAVE_KEY).ok().flatten());
        if let Some(saved) = saved {
            match Game::load(&saved).and_then(|game| self.install_game(game)) {
                Ok(()) => {
                    self.message = "Resumed saved rulebook hot-seat game in this browser.".into()
                }
                Err(e) => {
                    self.message =
                        format!("Saved game could not be loaded: {e} Original save was preserved.")
                }
            }
        }
    }

    pub(super) fn clear_game(&mut self) {
        self.game = None;
        if let Some(storage) = web_sys::window().unwrap().local_storage().ok().flatten() {
            let _ = storage.remove_item(SAVE_KEY);
        }
    }

    fn issue(&mut self, order: Order) -> Result<(), &'static str> {
        self.game
            .as_mut()
            .ok_or("Start a game first.")?
            .apply(order)?;
        self.project_game();
        self.save_game();
        Ok(())
    }

    pub(super) fn play_action(&mut self, action: &str) {
        let result = self.execute_play_action(action);
        match result {
            Ok(()) => {
                self.close();
                self.render();
            }
            Err(error) => {
                if let Some(alert) = document().get_element_by_id("play-error") {
                    alert.set_text_content(Some(error));
                } else {
                    self.message = error.into();
                    self.render();
                }
            }
        }
    }

    fn execute_play_action(&mut self, action: &str) -> Result<(), &'static str> {
        if action == "play-start" {
            let pieces = self
                .counters
                .iter()
                .map(|c| {
                    Ok(Piece {
                        id: c.id,
                        identity: c.identity.clone(),
                        side: identity_side(&self.inventory.find(&c.identity).unwrap().side)?,
                        hex: c.hex.ok_or("Place every counter before starting.")?,
                        supplies: None,
                        eliminated: false,
                        dug_in: false,
                    })
                })
                .collect::<Result<Vec<_>, &'static str>>()?;
            return self.install_game(Game::new(Setup {
                name: input("play-name"),
                calendar: Calendar::new(self.turn as u16, self.stage as u8)?,
                first: first(),
                pieces,
                seed: input("play-seed")
                    .parse()
                    .map_err(|_| "Dice seed must be a whole number from 0 to 4294967295.")?,
            })?);
        }
        if action == "play-import" {
            return self.install_game(Game::load(&text("play-save"))?);
        }
        let order = match action {
            "play-roll" => Order::Roll {},
            "play-concede" => Order::Concede {},
            "play-end" => Order::EndTurn {
                note: text("play-note"),
            },
            "play-stage" => Order::BeginStage {
                first: first(),
                note: text("play-note"),
            },
            other => {
                let id = self.selected.ok_or("Select a counter first.")?;
                let note = text("play-note");
                match other {
                    "play-move" => Order::Move {
                        id,
                        to: input("play-destination")
                            .trim()
                            .parse()
                            .map_err(|_| "Use a section hex, such as C4020.")?,
                        fuel: if input("play-fuel-cost").trim().is_empty() {
                            None
                        } else {
                            Some(FuelPoints(points(&input("play-fuel-cost"))?))
                        },
                        note,
                    },
                    "play-combat" => Order::Combat {
                        attacker: id,
                        defender: document()
                            .get_element_by_id("play-target")
                            .unwrap()
                            .unchecked_into::<HtmlSelectElement>()
                            .value()
                            .parse()
                            .map_err(|_| "Choose an enemy counter.")?,
                        attacker_lost: document()
                            .get_element_by_id("play-attacker-lost")
                            .unwrap()
                            .unchecked_into::<HtmlInputElement>()
                            .checked(),
                        defender_lost: document()
                            .get_element_by_id("play-defender-lost")
                            .unwrap()
                            .unchecked_into::<HtmlInputElement>()
                            .checked(),
                        note,
                    },
                    "play-supplies" => Order::Supplies {
                        id,
                        supplies: Supplies {
                            fuel: FuelPoints(points(&input("play-fuel"))?),
                            ammunition: AmmunitionPoints(points(&input("play-ammo"))?),
                            stores: StoresPoints(points(&input("play-stores"))?),
                            water: WaterPoints(points(&input("play-water"))?),
                        },
                        note,
                    },
                    "play-digin" => Order::DigIn { id, note },
                    _ => return Err("Unknown order."),
                }
            }
        };
        self.issue(order)
    }
}

fn identity_side(side: &str) -> Result<Side, &'static str> {
    match side {
        "axis" => Ok(Side::Axis),
        "allied" => Ok(Side::Commonwealth),
        _ => Err("Only Axis and Commonwealth identities can enter this game."),
    }
}

pub(super) fn detail(piece: &Piece) -> String {
    let supplies = piece
        .supplies
        .map(|s| {
            format!(
                "Fuel {} · Ammunition {} · Stores {} · Water {}",
                quantity(s.fuel.0),
                quantity(s.ammunition.0),
                quantity(s.stores.0),
                quantity(s.water.0)
            )
        })
        .unwrap_or("Supplies not recorded — tracked off-board until entered".into());
    format!(
        "<p class=\"play-summary\">{} · {}{}</p><p>{supplies}</p>",
        side_name(piece.side),
        if piece.eliminated {
            "Eliminated"
        } else {
            "In play"
        },
        if piece.dug_in {
            " · Construction recorded"
        } else {
            ""
        }
    )
}

pub(super) fn dialog(b: &Board, name: &str) -> (&'static str, String) {
    let body = if name == "play" {
        if let Some(game) = &b.game {
            let log: String = game
                .log
                .iter()
                .rev()
                .take(100)
                .map(|line| format!("<li>{}</li>", escape(line)))
                .collect();
            let controls = if game.winner.is_some() {
                "<p>Game ended. Export the record below, or use New operation to clear it.</p>"
            } else if game.awaiting_stage {
                r#"<div class="dialog-actions"><button data-action="end">Begin next stage</button><button data-action="concede">Concede game</button></div>"#
            } else {
                r#"<div class="dialog-actions"><button data-action="play-roll">Roll D6</button><button data-action="end">Complete player portion</button><button data-action="concede">Concede game</button></div>"#
            };
            format!(
                r#"<h2>{}</h2><p class="play-summary">{}</p><p>{LIMITS}</p>{controls}<details><summary>Save / replace this game</summary><p>Autosaved in this browser after each accepted order. Copy this JSON to back it up; importing replaces the current game only after validation. Saves contain both players’ open information and notes.</p><label>Session JSON<textarea id="play-save" spellcheck="false">{}</textarea></label>{}</details><h3>Order log (latest 100)</h3><ol class="play-log">{log}</ol>"#,
                escape(game.name()),
                status(game),
                escape(&game.save()),
                submit("play-import", "Replace with imported game")
            )
        } else {
            format!(
                r#"<h2>Play on this board</h2><p>{LIMITS}</p><p>1. Use Reinforcements to add counters for both sides and place them.<br>2. Set the starting week/stage in Campaign calendar.<br>3. Agree the scenario setup and initiative using the rulebook, then start here. Supplies begin unknown, not zero. No AI or network opponent: pass this browser between players.</p><div class="play-fields">{}{}{}<p>Dice seed is public and reproducible, not a competitive secret.</p></div>{}<details><summary>Import a saved game</summary><label>Session JSON<textarea id="play-save" spellcheck="false"></textarea></label><button data-action="play-import">Import game</button></details>"#,
                field("play-name", "Operation / setup reference", ""),
                first_field(),
                field("play-seed", "Dice seed", "2026"),
                submit("play-start", "Start hot-seat game")
            )
        }
    } else if name == "play-end" {
        if b.game.as_ref().unwrap().awaiting_stage {
            format!("<h2>Begin next operations stage</h2><p>Both player portions are complete. Resolve all applicable phases in the rulebook, including weekly initiative/convoys when the week changes and stage initiative declaration/weather. This button does not execute them.</p>{}{}{}", first_field(), note_field(), submit("play-stage", "Begin next stage"))
        } else {
            format!("<h2>Complete player portion</h2><p>Confirm movement, combat, reactions, convoy/rail movement, repair, patrol and all other applicable phases are adjudicated. The other player acts next; only after both portions can the stage advance.</p>{}{}", note_field(), submit("play-end", "Confirm player portion complete"))
        }
    } else if name == "play-concede" {
        format!("<h2>Concede this game?</h2><p>The active player concedes and the other side wins. No more orders can be issued; the saved game and log remain available.</p>{}", submit("play-concede", "Confirm concession"))
    } else {
        let piece = b.selected.and_then(|id| b.play_piece(id)).unwrap();
        let fields = match name {
            "play-move" => format!("<h2>Move counter {}</h2><p>Adjudicate the entire route, terrain, CP, reactions and breakdowns first. Geometry membership does not establish legality. Enter the resulting destination. Blank fuel cost means fuel is tracked off-board; a specified cost requires a supply ledger.</p>{}{}", piece.id, field("play-destination", "Destination hex", &piece.hex.to_string()), field("play-fuel-cost", "Adjudicated fuel cost (optional)", "")),
            "play-combat" => {
                let options: String = b.game.as_ref().unwrap().pieces.iter().filter(|p| p.side != piece.side && !p.eliminated).map(|p| format!("<option value=\"{}\">{} · {} · counter {}</option>", p.id, escape(&b.inventory.find(&p.identity).unwrap().name), p.hex, p.id)).collect();
                format!("<h2>Record combat result</h2><p>Resolve range, fire, morale, retreat, TOE losses and expenditure with the rulebook. Record partial losses in the adjudication; check elimination only if the entire counter is removed. This does not calculate a combat result.</p><label>Enemy counter<select id=\"play-target\">{options}</select></label><label><input type=\"checkbox\" id=\"play-attacker-lost\" />Attacker eliminated</label><label><input type=\"checkbox\" id=\"play-defender-lost\" />Defender eliminated</label>")
            }
            "play-supplies" => {
                let s = piece.supplies;
                format!("<h2>Record supply ledger</h2><p>Enter the new absolute balances after resolving access, transfers and expenditure. This is an adjudicated ledger, not free resupply. Fractions such as 3/2 are exact.</p>{}{}{}{}", field("play-fuel", "Fuel points", &s.map(|v| quantity(v.fuel.0)).unwrap_or_default()), field("play-ammo", "Ammunition points", &s.map(|v| quantity(v.ammunition.0)).unwrap_or_default()), field("play-stores", "Stores points", &s.map(|v| quantity(v.stores.0)).unwrap_or_default()), field("play-water", "Water points", &s.map(|v| quantity(v.water.0)).unwrap_or_default()))
            }
            _ => "<h2>Record construction</h2><p>Confirm the rulebook requirements, expenditure and completion. This marker does not automatically grant fortification or combat benefits.</p>".into(),
        };
        format!("{fields}{}{}", note_field(), submit(name, "Record order"))
    };
    (
        "Rulebook hot-seat",
        format!("<div class=\"play-panel\">{body}</div>"),
    )
}
