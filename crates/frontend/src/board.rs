//! Review-board UI. Planning counters never enter the teaching simulation or wire contracts.
use crate::{catalog::Catalog, document, inventory::Inventory};
use cna_map::{
    geometry::{self, BoardPoint},
    hex::HexId,
    viewport::Viewport,
};
use std::cell::RefCell;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Element, Event, HtmlDialogElement, HtmlElement, HtmlInputElement, KeyboardEvent};

mod calendar;
mod view;

struct Counter {
    id: usize,
    identity: String,
    hex: Option<HexId>,
}

struct Board {
    inventory: Inventory,
    catalog: Catalog,
    counters: Vec<Counter>,
    next_id: usize,
    selected: Option<usize>,
    placing: bool,
    turn: usize,
    stage: usize,
    modal: Option<&'static str>,
    return_focus: Option<Element>,
    message: String,
    overlay_html: String,
}

impl Board {
    fn new() -> Self {
        let inventory =
            Inventory::parse(include_str!("../../../public/data/unit-identities.json")).unwrap();
        let mut catalog =
            Catalog::parse(include_str!("../../../public/data/unit-catalog.json")).unwrap();
        catalog.apply_identity_artwork(&inventory);
        Self {
            inventory,
            catalog,
            counters: Vec::new(),
            next_id: 1,
            selected: None,
            placing: false,
            turn: 1,
            stage: 1,
            modal: None,
            return_focus: None,
            message: "Review board · user-placed counters only · session only".into(),
            overlay_html: String::new(),
        }
    }

    fn selected(&self) -> Option<&Counter> {
        self.counters.iter().find(|c| Some(c.id) == self.selected)
    }

    fn render(&self) {
        let focused = document().active_element().and_then(|e| focus_selector(&e));
        document()
            .get_element_by_id("board-top")
            .unwrap()
            .set_inner_html(&view::top(self));
        document()
            .get_element_by_id("board-roster")
            .unwrap()
            .set_inner_html(&view::roster(self));
        document()
            .get_element_by_id("board-detail")
            .unwrap()
            .set_inner_html(&view::detail(self));
        document()
            .get_element_by_id("board-message")
            .unwrap()
            .set_text_content(Some(&self.message));
        let mode = document().get_element_by_id("board-placement").unwrap();
        mode.set_inner_html(if self.placing {
            r#"<span>Choose a position on the map, or pan and press Enter to place at the center.</span><button data-action="cancel-placement">Cancel</button>"#
        } else { "" });
        mode.set_attribute("data-active", if self.placing { "true" } else { "false" })
            .unwrap();
        if let Some(selector) = focused {
            if let Some(element) = document().query_selector(&selector).unwrap() {
                let _ = element.unchecked_ref::<HtmlElement>().focus();
            }
        }
    }

    fn open(&mut self, name: &'static str) {
        self.return_focus = document().active_element();
        self.modal = Some(name);
        let host = document().get_element_by_id("board-dialog").unwrap();
        host.set_inner_html(&view::dialog(self, name));
        host.query_selector("dialog")
            .unwrap()
            .unwrap()
            .unchecked_into::<HtmlDialogElement>()
            .show_modal()
            .unwrap();
    }

    fn close(&mut self) {
        self.modal = None;
        document()
            .get_element_by_id("board-dialog")
            .unwrap()
            .set_inner_html("");
        let return_focus = self.return_focus.take().and_then(|e| {
            if e.is_connected() {
                Some(e)
            } else {
                focus_selector(&e).and_then(|s| document().query_selector(&s).ok().flatten())
            }
        });
        if let Some(element) = return_focus {
            let _ = element.unchecked_ref::<HtmlElement>().focus();
        } else {
            let _ = document()
                .query_selector(".research-viewport")
                .unwrap()
                .unwrap()
                .unchecked_ref::<HtmlElement>()
                .focus();
        }
    }

    fn place(&mut self, point: BoardPoint) {
        if !self.placing {
            return;
        }
        let Some(hex) = geometry::hex_at(point) else {
            self.message =
                "Choose a position within a mapped section, not a chart or off-board box.".into();
            self.render();
            return;
        };
        if let Some(counter) = self
            .counters
            .iter_mut()
            .find(|c| Some(c.id) == self.selected)
        {
            counter.hex = Some(hex);
            self.placing = false;
            self.message = format!("Planning position {hex} · VASSAL-derived ID, not a certified printed alias. Terrain and movement are unverified.");
        }
        self.render();
    }

    fn inventory_results(&self, page: usize) {
        let query = document()
            .get_element_by_id("inventory-query")
            .unwrap()
            .unchecked_into::<HtmlInputElement>()
            .value();
        document()
            .get_element_by_id("inventory-results")
            .unwrap()
            .set_inner_html(&self.inventory.planning_results(&query, page));
    }

    fn activate(&mut self, element: &Element) -> Option<BoardPoint> {
        if let Some(id) = element
            .get_attribute("data-counter")
            .and_then(|id| id.parse().ok())
        {
            self.selected = Some(id);
            self.placing = false;
            self.render();
            return self.selected().and_then(|c| c.hex).map(geometry::center);
        }
        let action = element.get_attribute("data-action").unwrap_or_default();
        match action.as_str() {
            "help" => self.open("help"),
            "catalog" => self.open("catalog"),
            "reinforcements" => self.open("reinforcements"),
            "calendar" => self.open("calendar"),
            "reset" => self.open("reset"),
            "end" | "attack" | "resupply" | "digin" => self.open("unavailable"),
            "close" => self.close(),
            "confirm-reset" => {
                self.counters.clear();
                self.selected = None;
                self.placing = false;
                self.turn = 1;
                self.stage = 1;
                self.message =
                    "New review board · no historical deployment loaded · session only".into();
                self.close();
                self.render();
            }
            "deploy-identity" => {
                let id = element.get_attribute("data-identity").unwrap();
                self.inventory.find(&id)?;
                self.counters.push(Counter {
                    id: self.next_id,
                    identity: id,
                    hex: None,
                });
                self.selected = Some(self.next_id);
                self.next_id += 1;
                self.placing = true;
                self.message = "Place your planning counter. This does not establish a historical position or legal deployment.".into();
                self.close();
                self.render();
                let _ = document()
                    .query_selector(".research-viewport")
                    .unwrap()
                    .unwrap()
                    .unchecked_ref::<HtmlElement>()
                    .focus();
            }
            "move-counter" => {
                self.placing = true;
                self.render();
                let _ = document()
                    .query_selector(".research-viewport")
                    .unwrap()
                    .unwrap()
                    .unchecked_ref::<HtmlElement>()
                    .focus();
            }
            "cancel-placement" => {
                self.placing = false;
                self.render();
            }
            "remove-counter" => {
                self.counters.retain(|c| Some(c.id) != self.selected);
                self.selected = None;
                self.placing = false;
                self.render();
            }
            "locate" => return self.selected().and_then(|c| c.hex).map(geometry::center),
            "inventory-page" => {
                self.inventory_results(
                    element.get_attribute("data-page").unwrap().parse().unwrap(),
                );
                let _ = document()
                    .query_selector(".inventory-count")
                    .unwrap()
                    .unwrap()
                    .unchecked_ref::<HtmlElement>()
                    .focus();
            }
            "catalog-type" => {
                self.catalog
                    .select_type(&element.get_attribute("data-type").unwrap());
            }
            "catalog-record" => {
                self.catalog
                    .open_record(&element.get_attribute("data-record").unwrap());
            }
            "catalog-list" => self.catalog.back_to_list(),
            "catalog-page" => {
                self.catalog.update_results(
                    element.get_attribute("data-page").unwrap().parse().unwrap(),
                    true,
                );
                let _ = document()
                    .query_selector(".catalog-count")
                    .unwrap()
                    .unwrap()
                    .unchecked_ref::<HtmlElement>()
                    .focus();
            }
            "choose-turn" | "choose-stage" => {
                let value: usize = element
                    .get_attribute("data-value")
                    .unwrap()
                    .parse()
                    .unwrap();
                if action == "choose-turn" && (1..=111).contains(&value) {
                    self.turn = value;
                }
                if action == "choose-stage" && (1..=3).contains(&value) {
                    self.stage = value;
                }
                // Update in place so keyboard focus and the calendar's scroll position survive.
                for (selector, selected) in [
                    ("[data-action=choose-turn]", self.turn),
                    ("[data-action=choose-stage]", self.stage),
                ] {
                    let nodes = document().query_selector_all(selector).unwrap();
                    for i in 0..nodes.length() {
                        let node: Element = nodes.item(i).unwrap().unchecked_into();
                        node.set_attribute(
                            "aria-pressed",
                            if node.get_attribute("data-value") == Some(selected.to_string()) {
                                "true"
                            } else {
                                "false"
                            },
                        )
                        .unwrap();
                    }
                }
                document()
                    .get_element_by_id("calendar-selection")
                    .unwrap()
                    .set_text_content(Some(&format!(
                        "{} · Game-Turn {} · Operations Stage {}",
                        calendar::label(self.turn),
                        self.turn,
                        self.stage
                    )));
                document()
                    .get_element_by_id("board-top")
                    .unwrap()
                    .set_inner_html(&view::top(self));
            }
            "source-track" => {
                self.close();
                return Some(BoardPoint {
                    x: 11120.0,
                    y: 525.0,
                });
            }
            _ => {}
        }
        None
    }
}

thread_local! { static BOARD: RefCell<Board> = RefCell::new(Board::new()); }

pub(crate) fn place(point: BoardPoint) {
    BOARD.with(|b| b.borrow_mut().place(point));
}

pub(crate) fn overlay(viewport: Viewport) {
    BOARD.with(|b| {
        let mut b = b.borrow_mut();
        let Some(layer) = document().get_element_by_id("board-counters") else {
            return;
        };
        let html = view::counters(&b, viewport);
        let mut focused = None;
        if html != b.overlay_html {
            focused = document()
                .active_element()
                .filter(|e| layer.contains(Some(e)))
                .and_then(|e| focus_selector(&e));
            layer.set_inner_html(&html);
            b.overlay_html = html;
        }
        // Measure the rendered controls, not duplicated CSS sizes. Keep their
        // pointer attached to the counter even when the bar shifts at an edge.
        if let Some(actions) = layer.query_selector(".counter-actions").unwrap() {
            let anchor = layer
                .query_selector(".selected .counter-marker")
                .unwrap()
                .unwrap()
                .get_bounding_client_rect();
            let bounds = layer.get_bounding_client_rect();
            let bar = actions.get_bounding_client_rect();
            let tips = actions.query_selector_all(".board-tooltip").unwrap();
            let tip_height = (0..tips.length())
                .map(|i| {
                    tips.item(i)
                        .unwrap()
                        .unchecked_into::<Element>()
                        .get_bounding_client_rect()
                        .height()
                })
                .fold(0.0, f64::max);
            let center = anchor.left() + anchor.width() / 2.0;
            let mut left = (center - bar.width() / 2.0).clamp(
                bounds.left() + 8.0,
                (bounds.right() - bar.width() - 8.0).max(bounds.left() + 8.0),
            );
            let mut above =
                anchor.bottom() + 12.0 + bar.height() + 10.0 + tip_height > bounds.bottom() - 8.0;
            let mut top = if above {
                anchor.top() - 12.0 - bar.height()
            } else {
                anchor.bottom() + 12.0
            }
            .max(bounds.top() + 8.0);
            // Leave the panels usable: find free horizontal space for the bar
            // and tooltip instead of raising them over panel content.
            let panels = document().query_selector_all(".floating-panel").unwrap();
            let mut slots = vec![(bounds.left() + 8.0, bounds.right() - 8.0)];
            let mut panels_bottom = bounds.top();
            for i in 0..panels.length() {
                let panel = panels
                    .item(i)
                    .unwrap()
                    .unchecked_into::<Element>()
                    .get_bounding_client_rect();
                panels_bottom = panels_bottom.max(panel.bottom());
                let tooltip_top = if above { top - tip_height - 10.0 } else { top };
                let tooltip_bottom =
                    top + bar.height() + if above { 0.0 } else { tip_height + 10.0 };
                if tooltip_top < panel.bottom() + 8.0 && tooltip_bottom > panel.top() - 8.0 {
                    slots = slots
                        .into_iter()
                        .flat_map(|(start, end)| {
                            [
                                (start, end.min(panel.left() - 8.0)),
                                (start.max(panel.right() + 8.0), end),
                            ]
                        })
                        .filter(|(start, end)| end - start >= bar.width())
                        .collect();
                }
            }
            if let Some(free_left) = slots
                .into_iter()
                .map(|(start, end)| left.clamp(start, (end - bar.width()).max(start)))
                .min_by(|a, b| (a - left).abs().total_cmp(&(b - left).abs()))
            {
                left = free_left;
            } else {
                // Narrow layouts can have no bar-sized gap between the panels.
                top = panels_bottom + 8.0;
                above = false;
            }
            let gap = if above {
                anchor.top() - top - bar.height()
            } else {
                top - anchor.bottom()
            };
            let pointer = (center - left).clamp(12.0, bar.width() - 12.0);
            let dx = center - left - pointer;
            let dy = (gap + anchor.height() / 2.0 - 6.0).max(0.0);
            let leader = if gap > 12.0 || dx.abs() > 1.0 { dx.hypot(dy) } else { 0.0 };
            let angle = dx.atan2(dy).to_degrees() * if above { -1.0 } else { 1.0 };
            actions
                .set_attribute("data-placement", if above { "above" } else { "below" })
                .unwrap();
            actions
                .set_attribute(
                    "style",
                    &format!(
                        "left:{}px;top:{}px;--pointer-left:{pointer}px;--leader-height:{leader}px;--leader-angle:{angle}deg",
                        left - anchor.left(),
                        top - anchor.top()
                    ),
                )
                .unwrap();
        }
        // Restore focus only after clamping, so focus cannot scroll a clipped bar.
        if let Some(selector) = focused {
            if let Some(element) = document().query_selector(&selector).unwrap() {
                let _ = element.unchecked_ref::<HtmlElement>().focus();
            }
        }
    });
}

fn focus_selector(element: &Element) -> Option<String> {
    for attr in ["data-counter", "data-action"] {
        if let Some(value) = element.get_attribute(attr) {
            let region = element
                .closest("#board-roster, #board-detail, #board-top")
                .ok()
                .flatten()
                .map(|region| format!("#{} ", region.id()))
                .unwrap_or_default();
            let scope = if element.matches(".counter-marker").unwrap() {
                ".counter-marker"
            } else if element.closest(".counter-actions").ok().flatten().is_some() {
                ".counter-actions button"
            } else {
                "button"
            };
            return Some(format!("{region}{scope}[{attr}=\"{value}\"]"));
        }
    }
    None
}

pub fn mount() -> Result<(), JsValue> {
    let root = document().get_element_by_id("app").unwrap();
    root.set_inner_html(r#"<main class="board-workspace" aria-label="Campaign for North Africa review board"><div id="board-top"></div><details class="floating-panel order-panel" open><summary>Order of battle</summary><div id="board-roster"></div></details><details class="floating-panel unit-panel" open><summary>Unit information</summary><div id="board-detail"></div></details><div id="board-placement"></div><p id="board-message" role="status"></p><div id="board-dialog"></div></main>"#);
    crate::map::mount_research()?;
    BOARD.with(|b| b.borrow().render());
    let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|event: web_sys::MouseEvent| {
        let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        if let Some(control) = target.closest("[data-action], [data-counter]").unwrap() {
            let focus = BOARD.with(|b| b.borrow_mut().activate(&control));
            if let Some(point) = focus {
                crate::map::focus_board(point);
            } else {
                crate::map::refresh_board();
            }
        } else if target.tag_name() == "DIALOG" {
            let r = target.get_bounding_client_rect();
            if f64::from(event.client_x()) < r.left()
                || f64::from(event.client_x()) > r.right()
                || f64::from(event.client_y()) < r.top()
                || f64::from(event.client_y()) > r.bottom()
            {
                BOARD.with(|b| b.borrow_mut().close());
            }
        }
    });
    root.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    click.forget();
    let toggle = Closure::<dyn FnMut(Event)>::new(|_| crate::map::refresh_board());
    root.add_event_listener_with_callback_and_bool(
        "toggle",
        toggle.as_ref().unchecked_ref(),
        true,
    )?;
    toggle.forget();
    let input = Closure::<dyn FnMut(Event)>::new(|event: Event| {
        let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        BOARD.with(|b| {
            let b = b.borrow();
            if target.has_attribute("data-inventory-filter") && event.type_() == "input" {
                b.inventory_results(0);
            }
            if target.has_attribute("data-catalog-filter")
                && matches!(
                    (event.type_().as_str(), target.tag_name().as_str()),
                    ("input", "INPUT") | ("change", "SELECT")
                )
            {
                b.catalog.update_results(0, false);
            }
        });
    });
    root.add_event_listener_with_callback("input", input.as_ref().unchecked_ref())?;
    root.add_event_listener_with_callback("change", input.as_ref().unchecked_ref())?;
    input.forget();
    let keyboard = Closure::<dyn FnMut(KeyboardEvent)>::new(|event: KeyboardEvent| {
        if event.key() == "Tab" {
            if let Some(dialog) = document().query_selector("dialog[open]").unwrap() {
                let nodes = dialog
                    .query_selector_all("button:not([disabled]), input, select, summary, a[href]")
                    .unwrap();
                let controls: Vec<Element> = (0..nodes.length())
                    .map(|i| nodes.item(i).unwrap().unchecked_into::<Element>())
                    .filter(|e| e.get_bounding_client_rect().width() > 0.0)
                    .collect();
                if let (Some(first), Some(last), Some(active)) = (
                    controls.first(),
                    controls.last(),
                    document().active_element(),
                ) {
                    let wrap = if event.shift_key() && active == *first {
                        Some(last)
                    } else if !event.shift_key() && active == *last {
                        Some(first)
                    } else {
                        None
                    };
                    if let Some(next) = wrap {
                        event.prevent_default();
                        let _ = next.unchecked_ref::<HtmlElement>().focus();
                    }
                }
            }
        }
        if event.key() == "Escape" {
            event.prevent_default();
            BOARD.with(|b| {
                let mut b = b.borrow_mut();
                if b.modal.is_some() {
                    b.close();
                } else {
                    b.placing = false;
                    b.render();
                }
            });
            crate::map::refresh_board();
        }
    });
    root.add_event_listener_with_callback("keydown", keyboard.as_ref().unchecked_ref())?;
    keyboard.forget();
    Ok(())
}
