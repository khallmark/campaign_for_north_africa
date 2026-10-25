mod board;
mod catalog;
mod catalog_view;
mod inspect;
mod inventory;
mod map;
mod motion;
mod movement;
mod view;

use cna_sim::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Element, HtmlDialogElement, KeyboardEvent, MouseEvent};

struct Deployment {
    role_id: String,
    identity_id: Option<String>,
    side: String,
}

struct App {
    game: State,
    catalog: catalog::Catalog,
    inventory: inventory::Inventory,
    deployment: Option<Deployment>,
    selected: String,
    mode: &'static str,
    grid: bool,
    supplies: bool,
    zoom: f64,
    modal: Option<&'static str>,
    return_focus: Option<String>,
    target: String,
    message: String,
    error: bool,
    motions: HashMap<String, map::UnitMotion>,
}

impl App {
    fn new() -> Self {
        let inventory =
            inventory::Inventory::parse(include_str!("../../../public/data/unit-identities.json"))
                .expect("The identity inventory must match schema v1");
        let mut catalog =
            catalog::Catalog::parse(include_str!("../../../public/data/unit-catalog.json"))
                .expect("The bundled catalog must match schema v1");
        catalog.apply_identity_artwork(&inventory);
        Self {
            game: new_game(),
            catalog,
            inventory,
            deployment: None,
            selected: "7arm".into(),
            mode: "inspect",
            grid: true,
            supplies: false,
            zoom: 1.0,
            modal: None,
            return_focus: None,
            target: String::new(),
            message: "Select a formation, then issue an order. Your first objective is Tobruk."
                .into(),
            error: false,
            motions: HashMap::new(),
        }
    }

    fn apply(&mut self, result: ActionResult) {
        self.message = result.message;
        self.error = !result.ok;
        if self.game.outcome.is_some() {
            self.modal = Some("outcome");
        }
        if self.selected().is_none() {
            self.selected = self
                .game
                .units
                .iter()
                .find(|u| u.side == "allied" && u.hp > 0)
                .map(|u| u.id.clone())
                .unwrap_or_default();
        }
    }

    fn activate(&mut self, element: &Element) {
        if element.has_attribute("disabled") {
            return;
        }
        if let Some(id) = element.get_attribute("data-unit") {
            let Some(unit) = self.game.units.iter().find(|u| u.id == id) else {
                return;
            };
            if self.mode == "support" {
                let result = support_unit(&mut self.game, &self.selected, &id);
                self.apply(result);
            } else if self.mode == "attack" && unit.side == "axis" {
                if !can_attack(&self.game, self.selected(), Some(unit)) {
                    self.apply(ActionResult { ok: false, message: "That target is outside this role's attack domain/range, or needs 2 CP, water and 20 ammunition.".into() });
                } else {
                    self.target = id;
                    self.modal = Some("combat");
                }
            } else {
                self.message = format!(
                    "{} {}",
                    unit.name,
                    if unit.side == "allied" {
                        "selected. Choose an order in the formation panel."
                    } else {
                        "spotted. Enemy formations are controlled by the Axis commander."
                    }
                );
                self.selected = id;
                self.mode = "inspect";
                self.error = false;
            }
        } else if let Some(hex) = element.get_attribute("data-hex") {
            if let Some((c, r)) = hex.split_once(',') {
                if let (Ok(c), Ok(r)) = (c.parse(), r.parse()) {
                    let result = if self.mode == "deploy" {
                        let Some(deployment) = &self.deployment else {
                            return;
                        };
                        let result = deploy_unit(
                            &mut self.game,
                            &deployment.role_id,
                            &deployment.side,
                            Hex { c, r },
                        );
                        if result.ok {
                            let unit = self.game.units.last_mut().unwrap();
                            unit.identity_id = deployment.identity_id.clone();
                            if let Some(identity) = unit
                                .identity_id
                                .as_deref()
                                .and_then(|id| self.inventory.find(id))
                            {
                                unit.name = identity.name.clone();
                            }
                            self.selected = unit.id.clone();
                            self.mode = "inspect";
                            self.deployment = None;
                        }
                        result
                    } else {
                        move_unit(&mut self.game, &self.selected, Hex { c, r })
                    };
                    self.apply(result);
                }
            }
        } else {
            match element
                .get_attribute("data-action")
                .as_deref()
                .unwrap_or("")
            {
                "help" => self.modal = Some("help"),
                "catalog" => self.modal = Some("catalog"),
                "reinforcements" => self.modal = Some("reinforcements"),
                "deploy-role" | "deploy-identity" => {
                    let identity_id = element.get_attribute("data-identity");
                    let role_id = identity_id
                        .as_deref()
                        .and_then(|id| self.inventory.find(id))
                        .and_then(|entry| entry.role_id.clone())
                        .or_else(|| element.get_attribute("data-role"));
                    let Some(role_id) = role_id.filter(|id| role(id).is_some()) else {
                        return;
                    };
                    let side = document()
                        .get_element_by_id("deployment-side")
                        .unwrap()
                        .unchecked_into::<web_sys::HtmlSelectElement>()
                        .value();
                    self.message = format!(
                        "Choose an empty {} hex for this sandbox reinforcement.",
                        role(&role_id).unwrap().domain_name().to_lowercase()
                    );
                    self.deployment = Some(Deployment {
                        role_id,
                        identity_id,
                        side,
                    });
                    self.mode = "deploy";
                    self.modal = None;
                    self.error = false;
                }
                "inventory-page" => {
                    let page = element.get_attribute("data-page").unwrap().parse().unwrap();
                    self.inventory.update_results(page);
                    if let Some(heading) = document().query_selector(".inventory-count").unwrap() {
                        let _ = heading.unchecked_ref::<web_sys::HtmlElement>().focus();
                    }
                    return;
                }
                "catalog-type" => {
                    self.catalog
                        .select_type(&element.get_attribute("data-type").unwrap());
                    return;
                }
                "catalog-page" => {
                    let page = element.get_attribute("data-page").unwrap().parse().unwrap();
                    self.catalog.update_results(page);
                    let heading = document()
                        .query_selector(".catalog-count")
                        .unwrap()
                        .unwrap();
                    let _ = heading.unchecked_ref::<web_sys::HtmlElement>().focus();
                    return;
                }
                "reset" => self.modal = Some("reset"),
                "outcome" => self.modal = Some("outcome"),
                "close" => self.modal = None,
                "confirm-reset" => {
                    self.game = new_game();
                    self.selected = "7arm".into();
                    self.mode = "inspect";
                    self.modal = None;
                    self.zoom = 1.0;
                    self.error = false;
                    self.motions.clear();
                    self.deployment = None;
                    self.message = "New orders received. Advance west and secure Tobruk.".into();
                }
                "grid" => self.grid = !self.grid,
                "supplies" => self.supplies = !self.supplies,
                "zoom-in" => self.zoom = (self.zoom + 0.25).min(1.75),
                "zoom-out" => self.zoom = (self.zoom - 0.25).max(1.0),
                "zoom-reset" => self.zoom = 1.0,
                "cancel" => {
                    self.mode = "inspect";
                    self.deployment = None;
                }
                "support" => {
                    self.mode = if self.mode == "support" {
                        "inspect"
                    } else {
                        "support"
                    }
                }
                "move" => {
                    self.mode = if self.mode == "move" {
                        "inspect"
                    } else {
                        "move"
                    }
                }
                "attack" => {
                    self.mode = if self.mode == "attack" {
                        "inspect"
                    } else {
                        "attack"
                    }
                }
                "digin" => {
                    let r = dig_in(&mut self.game, &self.selected);
                    self.apply(r);
                }
                "resupply" => {
                    let r = resupply(&mut self.game, &self.selected);
                    self.apply(r);
                }
                "confirm-attack" => {
                    self.modal = None;
                    self.mode = "inspect";
                    let r = attack(&mut self.game, &self.selected, &self.target, "allied");
                    self.apply(r);
                }
                "end" => {
                    self.mode = "inspect";
                    let r = end_stage(&mut self.game);
                    self.apply(r);
                }
                _ => {}
            }
        }
        self.render();
    }

    fn render(&mut self) {
        let doc = document();
        let root = doc.get_element_by_id("app").unwrap();
        let focused = doc.active_element().and_then(|e| focus_selector(&e));
        let old_dialog = root.query_selector("dialog").unwrap();
        if self.modal.is_some() && old_dialog.is_none() {
            self.return_focus = focused.clone();
        }

        // Preserve the whole SVG subtree, including keyed units, focus, and animation state.
        let old_map = root.query_selector(".battle-map").unwrap();
        if let Some(svg) = &old_map {
            svg.remove();
        }
        root.set_inner_html(&view::page(self));
        let placeholder = root.query_selector("#map-slot").unwrap().unwrap();
        if let Some(svg) = old_map {
            placeholder.replace_with_with_node_1(&svg).unwrap();
        } else {
            placeholder.set_outer_html(&map::base());
        }
        map::update(self);

        if self.modal.is_some() {
            let dialog: HtmlDialogElement = root
                .query_selector("dialog")
                .unwrap()
                .unwrap()
                .unchecked_into();
            dialog.show_modal().unwrap();
        } else {
            let restore = self.return_focus.take().or(focused);
            if let Some(selector) = restore {
                let element = root.query_selector(&selector).unwrap().or_else(|| {
                    root.query_selector(&format!("g[data-unit=\"{}\"]", self.selected))
                        .unwrap()
                });
                if let Some(element) = element {
                    // SVG focus is supported by the browser, but web-sys exposes it on HtmlElement.
                    let options = web_sys::FocusOptions::new();
                    options.set_prevent_scroll(true);
                    let _ = element
                        .unchecked_ref::<web_sys::HtmlElement>()
                        .focus_with_options(&options);
                }
            }
        }
    }
}

thread_local! {
    static APP: RefCell<App> = RefCell::new(App::new());
    static FRAME_PENDING: Cell<bool> = const { Cell::new(false) };
}

fn request_frame() {
    if FRAME_PENDING.with(|pending| pending.replace(true)) {
        return;
    }
    let callback = Closure::once_into_js(|_: f64| {
        FRAME_PENDING.with(|pending| pending.set(false));
        if APP.with(|app| map::animate(&mut app.borrow_mut())) {
            request_frame();
        }
    });
    web_sys::window()
        .unwrap()
        .request_animation_frame(callback.unchecked_ref())
        .unwrap();
}

fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}
fn now() -> f64 {
    web_sys::window().unwrap().performance().unwrap().now()
}
fn focus_selector(e: &Element) -> Option<String> {
    // Prefix distinguishes the map unit from its roster button.
    for attr in ["data-action", "data-unit", "data-hex"] {
        if let Some(value) = e.get_attribute(attr) {
            return Some(format!(
                "{}[{attr}=\"{value}\"]",
                e.tag_name().to_lowercase()
            ));
        }
    }
    None
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    board::mount()
}

// Retained teaching implementation, deliberately not mounted on the full board.
#[allow(dead_code)]
fn mount_training() -> Result<(), JsValue> {
    APP.with(|app| app.borrow_mut().render());
    map::mount_research()?;
    let root = document().get_element_by_id("app").unwrap();
    let click = Closure::<dyn FnMut(MouseEvent)>::new(|event: MouseEvent| {
        let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        if let Some(element) = target
            .closest("[data-action], [data-unit], [data-hex]")
            .unwrap()
        {
            APP.with(|app| app.borrow_mut().activate(&element));
        } else if target.tag_name() == "DIALOG" {
            let rect = target.get_bounding_client_rect();
            let (x, y) = (event.client_x() as f64, event.client_y() as f64);
            if x < rect.left() || x > rect.right() || y < rect.top() || y > rect.bottom() {
                APP.with(|app| {
                    let mut app = app.borrow_mut();
                    app.modal = None;
                    app.render();
                });
            }
        }
    });
    root.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    click.forget();
    let input = Closure::<dyn FnMut(web_sys::Event)>::new(|event: web_sys::Event| {
        if event.type_() == "input"
            && event
                .target()
                .and_then(|t| t.dyn_into::<Element>().ok())
                .is_some_and(|target| target.has_attribute("data-inventory-filter"))
        {
            APP.with(|app| app.borrow().inventory.update_results(0));
            return;
        }
        if event
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .is_some_and(|target| {
                target.has_attribute("data-catalog-filter")
                    && matches!(
                        (event.type_().as_str(), target.tag_name().as_str()),
                        ("input", "INPUT") | ("change", "SELECT")
                    )
            })
        {
            APP.with(|app| app.borrow().catalog.update_results(0));
        }
    });
    root.add_event_listener_with_callback("input", input.as_ref().unchecked_ref())?;
    root.add_event_listener_with_callback("change", input.as_ref().unchecked_ref())?;
    input.forget();
    let keyboard = Closure::<dyn FnMut(KeyboardEvent)>::new(|event: KeyboardEvent| {
        let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        if (event.key() == "Enter" || event.key() == " ")
            && target.matches("g[role=button]").unwrap()
        {
            event.prevent_default();
            APP.with(|app| app.borrow_mut().activate(&target));
        }
        if event.key() == "Escape" {
            event.prevent_default();
            APP.with(|app| {
                let mut app = app.borrow_mut();
                if app.modal.is_some() {
                    app.modal = None;
                } else {
                    app.mode = "inspect";
                }
                app.render();
            });
        }
    });
    root.add_event_listener_with_callback("keydown", keyboard.as_ref().unchecked_ref())?;
    keyboard.forget();
    Ok(())
}
