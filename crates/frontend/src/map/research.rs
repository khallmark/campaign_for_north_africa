//! Full-board viewport. Planning overlays are separate from simulation state.
use crate::document;
use cna_map::{
    geometry::BoardPoint,
    viewport::{ScreenPoint, Viewport},
};
use serde::Deserialize;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{
    Element, Event, HtmlElement, KeyboardEvent, PointerEvent, WheelEvent, XmlHttpRequest,
};

#[derive(Deserialize)]
struct Manifest {
    board_width: u32,
    board_height: u32,
    tile_size: u32,
    levels: Vec<Level>,
}

#[derive(Deserialize)]
struct Level {
    z: u32,
    width: u32,
    height: u32,
    columns: u32,
    rows: u32,
}

struct Research {
    root: Element,
    surface: Element,
    board: Element,
    status: Element,
    readout: Element,
    board_size: [f64; 2],
    viewport: Option<Viewport>,
    manifest: Option<Manifest>,
    tiles: BTreeMap<(u32, u32, u32), Element>,
    pointers: BTreeMap<i32, ScreenPoint>,
    press: Option<ScreenPoint>,
    dragged: bool,
}

impl Research {
    fn fit(&self) -> f64 {
        let rect = self.surface.get_bounding_client_rect();
        (rect.width() / self.board_size[0]).min(rect.height() / self.board_size[1])
    }

    fn resize(&mut self) {
        if !self.root.has_attribute("open") {
            return;
        }
        let rect = self.surface.get_bounding_client_rect();
        if rect.width() == 0.0 || rect.height() == 0.0 {
            return;
        }
        let (center, zoom) = self
            .viewport
            .map(|v| (v.center(), v.zoom().max(self.fit())))
            .unwrap_or((
                BoardPoint {
                    x: self.board_size[0] / 2.0,
                    y: self.board_size[1] / 2.0,
                },
                self.fit().max(rect.height() / self.board_size[1]).max(0.25),
            ));
        let mut viewport = Viewport::new(
            center,
            zoom,
            [rect.width(), rect.height()],
            web_sys::window().unwrap().device_pixel_ratio(),
        )
        .unwrap();
        viewport.clamp_to_board(self.board_size).unwrap();
        self.viewport = Some(viewport);
        self.render();
    }

    fn point(&self, x: f64, y: f64) -> ScreenPoint {
        let rect = self.surface.get_bounding_client_rect();
        ScreenPoint {
            x: x - rect.left(),
            y: y - rect.top(),
        }
    }

    fn transform(&mut self, from: ScreenPoint, to: ScreenPoint, zoom: f64) {
        let Some(old) = self.viewport else {
            return;
        };
        let anchor = old.to_board(from);
        let zoom = zoom.clamp(self.fit(), 4.0);
        let [w, h] = old.css_size();
        let mut viewport = Viewport::new(
            BoardPoint {
                x: anchor.x - (to.x - w / 2.0) / zoom,
                y: anchor.y - (to.y - h / 2.0) / zoom,
            },
            zoom,
            [w, h],
            old.device_pixel_ratio(),
        )
        .unwrap();
        viewport.clamp_to_board(self.board_size).unwrap();
        self.viewport = Some(viewport);
        self.render();
    }

    fn control(&mut self, action: &str) {
        let Some(v) = self.viewport else {
            return;
        };
        let [w, h] = v.css_size();
        let from = ScreenPoint {
            x: w / 2.0,
            y: h / 2.0,
        };
        let mut to = from;
        let zoom = match action {
            "in" => v.zoom() * 1.25,
            "out" => v.zoom() / 1.25,
            "gameplay" => 1.0,
            "overview" => self.fit(),
            "left" => {
                to.x += 100.0;
                v.zoom()
            }
            "right" => {
                to.x -= 100.0;
                v.zoom()
            }
            "up" => {
                to.y += 100.0;
                v.zoom()
            }
            "down" => {
                to.y -= 100.0;
                v.zoom()
            }
            _ => return,
        };
        self.transform(from, to, zoom);
    }

    fn range(&self, level: &Level) -> [u32; 4] {
        let v = self.viewport.unwrap();
        let [w, h] = v.css_size();
        let low = v.to_board(ScreenPoint { x: 0.0, y: 0.0 });
        let high = v.to_board(ScreenPoint { x: w, y: h });
        let sx = f64::from(level.width) / self.board_size[0] / 512.0;
        let sy = f64::from(level.height) / self.board_size[1] / 512.0;
        [
            ((low.x * sx).floor() - 1.0).max(0.0) as u32,
            ((high.x * sx).ceil() as u32).min(level.columns - 1),
            ((low.y * sy).floor() - 1.0).max(0.0) as u32,
            ((high.y * sy).ceil() as u32).min(level.rows - 1),
        ]
    }

    fn render(&mut self) {
        if !self.root.has_attribute("open") {
            return;
        }
        let Some(v) = self.viewport else {
            return;
        };
        let center = v.center();
        let origin = v.to_screen(BoardPoint { x: 0.0, y: 0.0 });
        self.board
            .set_attribute(
                "style",
                &format!(
                    "width:{}px;height:{}px;transform:translate({}px,{}px) scale({})",
                    self.board_size[0],
                    self.board_size[1],
                    origin.x,
                    origin.y,
                    v.zoom()
                ),
            )
            .unwrap();
        for (name, value) in [
            ("center-x", center.x),
            ("center-y", center.y),
            ("zoom", v.zoom()),
        ] {
            self.surface
                .set_attribute(&format!("data-{name}"), &value.to_string())
                .unwrap();
        }
        self.readout.set_text_content(Some(&format!(
            "{:.0}% · Full source board",
            v.zoom() * 100.0
        )));
        crate::board::overlay(v);
        let Some(manifest) = &self.manifest else {
            return;
        };
        let desired = v.zoom() * v.device_pixel_ratio();
        let mut index = manifest
            .levels
            .iter()
            .position(|l| {
                f64::from(l.width) / self.board_size[0] >= desired
                    && f64::from(l.height) / self.board_size[1] >= desired
            })
            .unwrap_or(manifest.levels.len() - 1);
        // A DPR-aware detail choice is subordinate to the live tile budget.
        // Retire the prior level before requesting the new one.
        while index > 0 {
            let [x0, x1, y0, y1] = self.range(&manifest.levels[index]);
            if (x1 - x0 + 1) * (y1 - y0 + 1) <= 100 {
                break;
            }
            index -= 1;
        }
        let level = &manifest.levels[index];
        let [x0, x1, y0, y1] = self.range(level);
        self.surface
            .set_attribute("data-level", &level.z.to_string())
            .unwrap();
        self.tiles.retain(|&(z, x, y), node| {
            let keep = z == level.z && x >= x0 && x <= x1 && y >= y0 && y <= y1;
            if !keep {
                node.remove();
            }
            keep
        });
        for y in y0..=y1 {
            for x in x0..=x1 {
                let key = (level.z, x, y);
                if self.tiles.contains_key(&key) {
                    continue;
                }
                let node = document().create_element("div").unwrap();
                node.set_class_name("research-tile");
                let sx = self.board_size[0] / f64::from(level.width);
                let sy = self.board_size[1] / f64::from(level.height);
                node.set_attribute(
                    "style",
                    &format!(
                        "left:{}px;top:{}px;width:{}px;height:{}px",
                        f64::from(x * 512) * sx,
                        f64::from(y * 512) * sy,
                        f64::from((level.width - x * 512).min(512)) * sx,
                        f64::from((level.height - y * 512).min(512)) * sy
                    ),
                )
                .unwrap();
                let placeholder = document().create_element("span").unwrap();
                placeholder.set_text_content(Some(&format!(
                    "Tile {}:{x}:{y} — loading or unavailable",
                    level.z
                )));
                node.append_child(&placeholder).unwrap();
                let image = document().create_element("img").unwrap();
                image.set_attribute("alt", "").unwrap();
                image.set_attribute("draggable", "false").unwrap();
                node.append_child(&image).unwrap();
                self.board.append_child(&node).unwrap();
                image
                    .set_attribute("src", &format!("/tiles/map/{}/{x}_{y}.png", level.z))
                    .unwrap();
                self.tiles.insert(key, node);
            }
        }
    }
}

fn pointer_geometry(points: &BTreeMap<i32, ScreenPoint>) -> (ScreenPoint, f64) {
    let mut points = points.values();
    let a = *points.next().unwrap();
    match points.next() {
        Some(b) => (
            ScreenPoint {
                x: (a.x + b.x) / 2.0,
                y: (a.y + b.y) / 2.0,
            },
            (a.x - b.x).hypot(a.y - b.y).max(1.0),
        ),
        None => (a, 1.0),
    }
}

fn request_manifest(state: Rc<RefCell<Research>>) -> Result<(), JsValue> {
    let xhr = XmlHttpRequest::new()?;
    xhr.open("GET", "/tiles/map/manifest.json")?;
    xhr.set_timeout(10_000);
    let request = xhr.clone();
    let loaded = Closure::<dyn FnMut(Event)>::new(move |_| {
        let mut state = state.borrow_mut();
        let parsed = request
            .status()
            .ok()
            .filter(|s| *s == 200)
            .and_then(|_| request.response_text().ok().flatten())
            .and_then(|text| serde_json::from_str::<Manifest>(&text).ok());
        if let Some(manifest) = parsed.filter(|m| {
            [f64::from(m.board_width), f64::from(m.board_height)] == state.board_size
                && m.tile_size == 512
                && !m.levels.is_empty()
                && m.levels.len() <= 16
                && m.levels.iter().enumerate().all(|(i, l)| {
                    l.z == i as u32
                        && l.width > 0
                        && l.height > 0
                        && l.width <= m.board_width
                        && l.height <= m.board_height
                        && l.columns == l.width.div_ceil(512)
                        && l.rows == l.height.div_ceil(512)
                        && (i == 0
                            || (l.width > m.levels[i - 1].width
                                && l.height >= m.levels[i - 1].height))
                })
        }) {
            if manifest.levels[0].columns == 1 && manifest.levels[0].rows == 1 {
                let overview = document().create_element("img").unwrap();
                overview.set_class_name("research-overview");
                overview.set_attribute("alt", "").unwrap();
                overview.set_attribute("draggable", "false").unwrap();
                overview.set_attribute("fetchpriority", "high").unwrap();
                state.board.prepend_with_node_1(&overview).unwrap();
                overview
                    .set_attribute("src", "/tiles/map/0/0_0.png")
                    .unwrap();
            }
            state.manifest = Some(manifest);
            state
                .surface
                .query_selector(".research-unavailable")
                .unwrap()
                .unwrap()
                .set_attribute("hidden", "")
                .unwrap();
            state
                .status
                .set_text_content(Some("Source map loaded · terrain and seams unverified"));
            state.render();
        } else {
            state.status.set_text_content(Some("Artwork unavailable · configure private-review tiles; public builds do not bundle source artwork."));
        }
        // One request per page; release its callback after completion.
        request.set_onloadend(None);
    });
    xhr.set_onloadend(Some(loaded.as_ref().unchecked_ref()));
    xhr.send()?;
    loaded.forget();
    Ok(())
}

pub fn mount() -> Result<(), JsValue> {
    if document().get_element_by_id("map-research").is_some() {
        return Ok(());
    }
    let root = document().create_element("section")?;
    root.set_id("map-research");
    root.set_attribute("open", "")?;
    root.set_inner_html(r#"<div class="research-controls" aria-label="Map controls"><button data-map-control="overview" aria-label="Fit full board"><svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M9 3H3v6m12-6h6v6M3 15v6h6m12-6v6h-6"/></svg></button><button data-map-control="gameplay" aria-label="Map detail scale">1:1</button><button data-map-control="out" aria-label="Zoom out map">−</button><button data-map-control="in" aria-label="Zoom in map">+</button></div>
      <p class="research-status" role="status">Checking local tile manifest…</p>
      <div class="research-viewport" tabindex="0" role="region" aria-label="Full-board map" aria-describedby="research-help"><div class="research-board"></div><div id="board-counters"></div><span class="research-unavailable">Source artwork is not bundled</span></div>
      <div class="research-footer"><p id="research-help">Drag to pan · scroll to zoom · arrows / + / − / Home</p><output class="research-readout"></output></div>"#);
    document()
        .query_selector(".board-workspace")?
        .unwrap()
        .prepend_with_node_1(&root)?;
    let surface = root.query_selector(".research-viewport")?.unwrap();
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../../../../public/data/map/sources.json")).unwrap();
    let state = Rc::new(RefCell::new(Research {
        surface: surface.clone(),
        board: root.query_selector(".research-board")?.unwrap(),
        status: root.query_selector(".research-status")?.unwrap(),
        readout: root.query_selector(".research-readout")?.unwrap(),
        board_size: [
            source["image"]["width"].as_f64().unwrap(),
            source["image"]["height"].as_f64().unwrap(),
        ],
        root: root.clone(),
        viewport: None,
        manifest: None,
        tiles: BTreeMap::new(),
        pointers: BTreeMap::new(),
        press: None,
        dragged: false,
    }));
    RESEARCH.with(|current| *current.borrow_mut() = Some(state.clone()));
    state.borrow_mut().resize();
    request_manifest(state.clone())?;
    let s = state.clone();
    let click = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        if let Some(action) = event
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|t| t.closest("[data-map-control]").ok().flatten())
            .and_then(|t| t.get_attribute("data-map-control"))
        {
            s.borrow_mut().control(&action);
        }
    });
    root.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    click.forget();
    for name in [
        "pointerdown",
        "pointermove",
        "pointerup",
        "pointercancel",
        "lostpointercapture",
    ] {
        let s = state.clone();
        let pointer = Closure::<dyn FnMut(PointerEvent)>::new(move |event: PointerEvent| {
            if event
                .target()
                .and_then(|t| t.dyn_into::<Element>().ok())
                .and_then(|t| t.closest("button").ok().flatten())
                .is_some()
            {
                return;
            }
            let mut state = s.borrow_mut();
            let id = event.pointer_id();
            let point = state.point(f64::from(event.client_x()), f64::from(event.client_y()));
            match event.type_().as_str() {
                "pointerdown" if event.button() == 0 && state.pointers.len() < 2 => {
                    event.prevent_default();
                    let _ = state.surface.unchecked_ref::<HtmlElement>().focus();
                    let _ = state.surface.set_pointer_capture(id);
                    if state.pointers.is_empty() {
                        state.press = Some(point);
                        state.dragged = false;
                    } else {
                        state.dragged = true;
                    }
                    state.pointers.insert(id, point);
                }
                "pointermove" if state.pointers.contains_key(&id) => {
                    event.prevent_default();
                    if state
                        .press
                        .is_some_and(|p| (p.x - point.x).hypot(p.y - point.y) > 4.0)
                    {
                        state.dragged = true;
                    }
                    let (from, distance) = pointer_geometry(&state.pointers);
                    state.pointers.insert(id, point);
                    let (to, new_distance) = pointer_geometry(&state.pointers);
                    let zoom = state.viewport.unwrap().zoom() * new_distance / distance;
                    state.transform(from, to, zoom);
                }
                "pointerup" | "pointercancel" | "lostpointercapture" => {
                    if event.type_() == "pointerup" && state.pointers.len() == 1 && !state.dragged {
                        if let Some(v) = state.viewport {
                            crate::board::place(v.to_board(point));
                        }
                    }
                    state.pointers.remove(&id);
                    state.render();
                }
                _ => {}
            }
            let _ = state.surface.set_attribute(
                "data-dragging",
                if state.pointers.is_empty() {
                    "false"
                } else {
                    "true"
                },
            );
        });
        surface.add_event_listener_with_callback(name, pointer.as_ref().unchecked_ref())?;
        pointer.forget();
    }
    let s = state.clone();
    let wheel = Closure::<dyn FnMut(WheelEvent)>::new(move |event: WheelEvent| {
        event.prevent_default();
        let mut state = s.borrow_mut();
        let Some(v) = state.viewport else {
            return;
        };
        let point = state.point(f64::from(event.client_x()), f64::from(event.client_y()));
        let delta = event.delta_y()
            * match event.delta_mode() {
                1 => 16.0,
                2 => v.css_size()[1],
                _ => 1.0,
            };
        state.transform(
            point,
            point,
            v.zoom() * 2.0_f64.powf((-delta / 600.0).clamp(-8.0, 8.0)),
        );
    });
    let options = web_sys::AddEventListenerOptions::new();
    options.set_passive(false);
    surface.add_event_listener_with_callback_and_add_event_listener_options(
        "wheel",
        wheel.as_ref().unchecked_ref(),
        &options,
    )?;
    wheel.forget();
    let s = state.clone();
    let keyboard = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
        if event.ctrl_key() || event.meta_key() || event.alt_key() {
            return;
        }
        if event
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .is_some_and(|t| t.tag_name() == "BUTTON")
        {
            return;
        }
        if event.key() == "Enter" {
            event.prevent_default();
            let mut state = s.borrow_mut();
            if let Some(v) = state.viewport {
                crate::board::place(v.center());
            }
            state.render();
            return;
        }
        let action = match event.key().as_str() {
            "ArrowLeft" => "left",
            "ArrowRight" => "right",
            "ArrowUp" => "up",
            "ArrowDown" => "down",
            "+" | "=" => "in",
            "-" => "out",
            "Home" => "overview",
            _ => return,
        };
        event.prevent_default();
        s.borrow_mut().control(action);
    });
    surface.add_event_listener_with_callback("keydown", keyboard.as_ref().unchecked_ref())?;
    keyboard.forget();
    for name in ["load", "error"] {
        let loaded = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
            if let Some(image) = event
                .target()
                .and_then(|t| t.dyn_into::<Element>().ok())
                .filter(|t| t.tag_name() == "IMG")
            {
                if event.type_() == "error" {
                    let _ = image.set_attribute("hidden", "");
                }
                if let Some(parent) = image.parent_element() {
                    let _ = parent.set_attribute(
                        "data-ready",
                        if event.type_() == "load" {
                            "true"
                        } else {
                            "false"
                        },
                    );
                }
            }
        });
        surface.add_event_listener_with_callback_and_bool(
            name,
            loaded.as_ref().unchecked_ref(),
            true,
        )?;
        loaded.forget();
    }
    let resize = Closure::<dyn FnMut(Event)>::new(move |_| state.borrow_mut().resize());
    web_sys::window()
        .unwrap()
        .add_event_listener_with_callback("resize", resize.as_ref().unchecked_ref())?;
    resize.forget();
    Ok(())
}

thread_local! { static RESEARCH: RefCell<Option<Rc<RefCell<Research>>>> = const { RefCell::new(None) }; }

pub fn refresh() {
    RESEARCH.with(|s| {
        if let Some(s) = &*s.borrow() {
            s.borrow_mut().render();
        }
    });
}

pub fn focus(point: BoardPoint) {
    RESEARCH.with(|s| {
        if let Some(s) = &*s.borrow() {
            let mut s = s.borrow_mut();
            if let Some(v) = s.viewport {
                let mut next = Viewport::new(
                    point,
                    v.zoom().max(0.4),
                    v.css_size(),
                    v.device_pixel_ratio(),
                )
                .unwrap();
                next.clamp_to_board(s.board_size).unwrap();
                s.viewport = Some(next);
                s.render();
            }
        }
    });
}
