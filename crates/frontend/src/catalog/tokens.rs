//! Read existing reviewed counter links on demand; never infer identity from names.
use crate::view::escape;
use serde::Deserialize;
use std::{cell::RefCell, collections::BTreeSet};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Element, Event, XmlHttpRequest};

#[derive(Deserialize)]
struct References {
    schema_version: u32,
    units: Vec<Unit>,
    counters: Vec<Counter>,
    token_assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Unit {
    id: String,
    source_record_ids: Vec<String>,
    counter_gpids: Vec<String>,
    token_link_status: String,
}

#[derive(Deserialize)]
struct Counter {
    gpid: String,
    canonical_unit_id: Option<String>,
    basic_image_id: Option<String>,
    link_status: String,
}

#[derive(Deserialize)]
struct Asset {
    id: String,
    exists: bool,
    private_review_path: Option<String>,
}

impl References {
    fn render(&self, record: &str) -> String {
        let mut seen = BTreeSet::new();
        let mut images = String::new();
        let mut linked = false;
        for unit in self.units.iter().filter(|unit| {
            unit.token_link_status == "linked"
                && unit.source_record_ids.iter().any(|id| id == record)
        }) {
            for counter in self.counters.iter().filter(|counter| {
                counter.canonical_unit_id.as_deref() == Some(&unit.id)
                    && unit.counter_gpids.contains(&counter.gpid)
                    && matches!(
                        counter.link_status.as_str(),
                        "reviewed_source_link" | "evidenced_palette_copy"
                    )
            }) {
                linked = true;
                let Some(asset) = self.token_assets.iter().find(|asset| {
                    Some(&asset.id) == counter.basic_image_id.as_ref() && asset.exists
                }) else {
                    continue;
                };
                let Some(path) = asset
                    .private_review_path
                    .as_deref()
                    .filter(|path| path.starts_with("/__source-tokens/"))
                else {
                    continue;
                };
                if !seen.insert(&asset.id) {
                    continue;
                }
                images.push_str(&format!(r#"<figure><div class="catalog-token-image" data-state="loading"><img data-source-token src="{}" alt="Original counter {}" /><span>Loading original token…</span></div><figcaption>{}<small>Source counter GPID {}</small></figcaption></figure>"#,
                    escape(path.trim_start_matches('/')), escape(&asset.id), escape(&asset.id), escape(&counter.gpid)));
            }
        }
        if images.is_empty() {
            if linked {
                "<p>Reviewed token links exist, but their source image assets are unavailable.</p>"
                    .into()
            } else {
                "<p>No reviewed original-token link is available for this record. Shared illustrations are not original counters.</p>".into()
            }
        } else {
            format!("<p>Original VASSAL base images from reviewed identity links; duplicate palette copies shown once. State overlays are not composed and do not indicate current strength. Source artwork requires the private-review server.</p><div class=\"catalog-tokens\">{images}</div>")
        }
    }
}

enum State {
    Unloaded,
    Loading,
    Ready(References),
    Failed,
}

thread_local! {
    static STATE: RefCell<State> = const { RefCell::new(State::Unloaded) };
}

fn render_current() {
    let Some(panel) = crate::document()
        .query_selector("#catalog-detail .catalog-original-tokens")
        .unwrap()
    else {
        return;
    };
    let record = panel.get_attribute("data-record").unwrap();
    let body = STATE.with(|state| match &*state.borrow() {
        State::Ready(references) => references.render(&record),
        State::Failed => "<p>Original-token references could not be loaded. Unit statistics remain available; reload the page to retry.</p>".into(),
        _ => "<p>Loading original-token references…</p>".into(),
    });
    panel.set_inner_html(&format!("<h4>Original counter tokens</h4>{body}"));
}

pub(super) fn load() {
    let start = STATE.with(|state| {
        let mut state = state.borrow_mut();
        if matches!(*state, State::Unloaded) {
            *state = State::Loading;
            true
        } else {
            false
        }
    });
    if start {
        // One delegated listener for the page, including tokens rendered after navigation.
        let images = Closure::<dyn FnMut(Event)>::new(|event: Event| {
            let Some(image) = event
                .target()
                .and_then(|target| target.dyn_into::<Element>().ok())
            else {
                return;
            };
            if !image.has_attribute("data-source-token") {
                return;
            }
            let wrapper = image.parent_element().unwrap();
            let loaded = event.type_() == "load";
            wrapper
                .set_attribute("data-state", if loaded { "loaded" } else { "unavailable" })
                .unwrap();
            if !loaded {
                wrapper
                    .query_selector("span")
                    .unwrap()
                    .unwrap()
                    .set_text_content(Some(
                        "Original token unavailable — private-review source assets are required.",
                    ));
            }
        });
        for event in ["load", "error"] {
            crate::document()
                .add_event_listener_with_callback_and_bool(
                    event,
                    images.as_ref().unchecked_ref(),
                    true,
                )
                .unwrap();
        }
        images.forget();
        if request().is_err() {
            STATE.with(|state| *state.borrow_mut() = State::Failed);
        }
    }
    render_current();
}

fn request() -> Result<(), JsValue> {
    let xhr = XmlHttpRequest::new()?;
    xhr.open("GET", "data/unit-reference.json")?;
    xhr.set_timeout(15_000);
    let request = xhr.clone();
    let loaded = Closure::<dyn FnMut(Event)>::new(move |_| {
        let references = request
            .status()
            .ok()
            .filter(|status| *status == 200)
            .and_then(|_| request.response_text().ok().flatten())
            .and_then(|json| serde_json::from_str::<References>(&json).ok())
            .filter(|references| references.schema_version == 1);
        STATE.with(|state| {
            *state.borrow_mut() = references.map(State::Ready).unwrap_or(State::Failed)
        });
        render_current();
        request.set_onloadend(None);
    });
    xhr.set_onloadend(Some(loaded.as_ref().unchecked_ref()));
    xhr.send()?;
    loaded.forget();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_tokens_require_consistent_reviewed_links_and_deduplicate_palette_copies() {
        let mut references: References =
            serde_json::from_str(include_str!("../../../../public/data/unit-reference.json"))
                .unwrap();
        let record = "oa-116-001-1st-armored-div-hq";
        let html = references.render(record);
        assert_eq!(html.matches("<img ").count(), 1);
        assert!(html.contains("src=\"__source-tokens/BR%201%20-%20none.svg\""));
        assert!(!references
            .render("oa-124-002-13th-light-aa-regt")
            .contains("<img "));
        let asset = references
            .token_assets
            .iter_mut()
            .find(|asset| asset.id == "BR 1 - none.svg")
            .unwrap();
        asset.exists = false;
        assert!(references
            .render(record)
            .contains("source image assets are unavailable"));
        references
            .token_assets
            .iter_mut()
            .find(|asset| asset.id == "BR 1 - none.svg")
            .unwrap()
            .exists = true;
        for counter in references
            .counters
            .iter_mut()
            .filter(|counter| counter.gpid == "3634" || counter.gpid == "3861")
        {
            counter.link_status = "unresolved".into();
        }
        assert!(!references.render(record).contains("<img "));
        for counter in references
            .counters
            .iter_mut()
            .filter(|counter| counter.gpid == "3634" || counter.gpid == "3861")
        {
            counter.link_status = "reviewed_source_link".into();
            counter.canonical_unit_id = Some("another-unit".into());
        }
        assert!(!references.render(record).contains("<img "));
    }
}
