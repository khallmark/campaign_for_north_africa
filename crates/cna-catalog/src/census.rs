//! Source inventory only: GPIDs and palette copies are not canonical unit identities.
//!
//! Trait decoding follows VASSAL 3.7.15's SequenceEncoder, BasicCommandEncoder,
//! BasicPiece and Embellishment (the version recorded in moduledata).
//! https://github.com/vassalengine/vassal/tree/3.7.15/vassal-app/src/main/java/VASSAL

use roxmltree::{Document, Node};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

#[derive(Debug, Serialize)]
pub struct Census {
    pub schema_version: u32,
    pub interpretation: &'static str,
    pub summary: Summary,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub slots: usize,
    pub distinct_entry_names: usize,
    pub palettes: BTreeMap<String, usize>,
    pub kinds: BTreeMap<String, usize>,
    pub missing_images: BTreeSet<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct Slot {
    pub gpid: String,
    pub entry_name: String,
    pub source_line: u32,
    pub palette: String,
    pub palette_path: Vec<String>,
    pub slot_kind: String,
    pub nationality_prefix: Option<String>,
    pub side: Option<String>,
    pub printed_name: String,
    pub basic_image: String,
    pub prototype_refs: BTreeSet<String>,
    pub images: Vec<ImageRef>,
    pub layers: Vec<Layer>,
    /// Same entry label AND base-image stem in the OC palette. Evidence only;
    /// distinct GPIDs remain separate even when this list has a single member.
    pub duplicate_palette_candidates: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ImageRef {
    pub name: String,
    pub exists: bool,
    pub origin: String,
    pub trait_kind: String,
}

#[derive(Debug, Serialize)]
pub struct Layer {
    pub origin: String,
    pub name: String,
    pub description: String,
    pub initial_state: String,
    pub first_level_value: String,
    pub levels: Vec<Level>,
}

#[derive(Debug, Serialize)]
pub struct Level {
    pub index: usize,
    pub image: String,
    pub label: String,
}

// VASSAL escapes the delimiter with ONE preceding backslash per nesting level;
// it does not use the usual odd/even backslash rule. Other backslashes survive.
fn sequence(text: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    for c in text.chars() {
        if c == delimiter && field.ends_with('\\') {
            field.pop();
            field.push(c);
        } else if c == delimiter {
            fields.push(std::mem::take(&mut field));
        } else {
            field.push(c);
        }
    }
    fields.push(field);
    for field in &mut fields {
        if field.len() > 1 && field.starts_with('\'') && field.ends_with('\'') {
            *field = field[1..field.len() - 1].to_owned();
        }
    }
    fields
}

fn field(fields: &[String], index: usize) -> &str {
    fields.get(index).map(String::as_str).unwrap_or("")
}

fn inspect_traits(
    definition: &str,
    origin: &str,
    prototypes: &BTreeMap<String, String>,
    stack: &mut Vec<String>,
    images: &Path,
    slot: &mut Slot,
) -> Result<(), String> {
    let command = sequence(definition, '/');
    if field(&command, 0) != "+" || command.len() != 4 {
        return Err(format!("{origin}: expected a VASSAL AddPiece definition"));
    }
    let mut remaining = command[2].clone();
    let mut state = command[3].clone();
    while !remaining.is_empty() {
        let types = sequence(&remaining, '\t');
        let states = sequence(&state, '\t');
        let fields = sequence(&types[0], ';');
        let kind = field(&fields, 0);
        match kind {
            "prototype" => {
                let name = field(&fields, 1);
                slot.prototype_refs.insert(name.to_owned());
                if stack.iter().any(|p| p == name) {
                    return Err(format!("prototype cycle: {} -> {name}", stack.join(" -> ")));
                }
                let definition = prototypes
                    .get(name)
                    .ok_or_else(|| format!("missing prototype: {name}"))?;
                stack.push(name.to_owned());
                inspect_traits(definition, name, prototypes, stack, images, slot)?;
                stack.pop();
            }
            "piece" if origin == "slot" => {
                slot.basic_image = field(&fields, 3).to_owned();
                slot.printed_name = field(&fields, 4).to_owned();
            }
            "emb2" => {
                if fields.len() < 20 {
                    return Err(format!("{origin}: truncated emb2 layer"));
                }
                let names = sequence(field(&fields, 16), ',');
                let labels = sequence(field(&fields, 17), ',');
                slot.layers.push(Layer {
                    origin: origin.to_owned(),
                    name: field(&fields, 19).to_owned(),
                    description: field(&fields, 30).to_owned(),
                    initial_state: field(&states, 0).to_owned(),
                    first_level_value: field(&fields, 24).to_owned(),
                    levels: names
                        .into_iter()
                        .enumerate()
                        .map(|(i, image)| Level {
                            index: i + 1,
                            image,
                            label: field(&labels, i).to_owned(),
                        })
                        .collect(),
                });
            }
            _ => {}
        }
        // Image-bearing fields in the supplied module: BasicPiece, layers,
        // movement flags and obscured counters. Scan the remaining trait fields
        // too, retaining literal references even when no corresponding file exists.
        let image_names: Vec<String> = match kind {
            "piece" => vec![field(&fields, 3).to_owned()],
            "emb2" => sequence(field(&fields, 16), ','),
            _ => fields
                .iter()
                .skip(1)
                .flat_map(|f| sequence(f, ','))
                .filter(|s| {
                    Path::new(s)
                        .extension()
                        .and_then(|x| x.to_str())
                        .is_some_and(|e| {
                            ["svg", "png", "gif", "jpg", "jpeg"]
                                .contains(&e.to_ascii_lowercase().as_str())
                        })
                })
                .collect(),
        };
        for name in image_names.into_iter().filter(|s| !s.is_empty()) {
            let path = Path::new(&name);
            let safe = path.components().all(|c| matches!(c, Component::Normal(_)));
            let exists = safe && images.join(path).is_file();
            if !slot
                .images
                .iter()
                .any(|i| i.name == name && i.origin == origin && i.trait_kind == kind)
            {
                slot.images.push(ImageRef {
                    name,
                    exists,
                    origin: origin.to_owned(),
                    trait_kind: kind.to_owned(),
                });
            }
        }
        remaining = field(&types, 1).to_owned();
        state = field(&states, 1).to_owned();
    }
    Ok(())
}

fn node_name<'a>(node: Node<'a, 'a>) -> Option<&'a str> {
    node.attribute("entryName")
        .or_else(|| node.attribute("name"))
        .or_else(|| node.attribute("mapName"))
}

/// Census a buildFile.xml, never a save file. Missing images are data, while
/// malformed XML, duplicate GPIDs and broken prototype chains fail the scan.
pub fn scan(xml: &str, images: &Path) -> Result<Census, String> {
    let doc = Document::parse(xml).map_err(|e| e.to_string())?;
    if !doc.root_element().has_tag_name("VASSAL.build.GameModule") {
        return Err("expected a VASSAL.build.GameModule buildFile.xml, not a save".to_owned());
    }
    let mut prototypes = BTreeMap::new();
    for node in doc
        .descendants()
        .filter(|n| n.has_tag_name("VASSAL.build.module.PrototypeDefinition"))
    {
        let name = node
            .attribute("name")
            .ok_or("prototype without name")?
            .to_owned();
        if prototypes
            .insert(name.clone(), node.text().unwrap_or("").to_owned())
            .is_some()
        {
            return Err(format!("duplicate prototype: {name}"));
        }
    }
    let mut slots = Vec::new();
    let mut gpids = BTreeSet::new();
    for node in doc
        .descendants()
        .filter(|n| n.has_tag_name("VASSAL.build.widget.PieceSlot"))
    {
        let gpid = node
            .attribute("gpid")
            .filter(|s| !s.is_empty())
            .ok_or("PieceSlot without GPID")?
            .to_owned();
        if !gpids.insert(gpid.clone()) {
            return Err(format!("duplicate GPID: {gpid}"));
        }
        let entry_name = node
            .attribute("entryName")
            .ok_or("PieceSlot without entryName")?
            .to_owned();
        let ancestors: Vec<_> = node
            .ancestors()
            .skip(1)
            .filter(|n| n.is_element())
            .collect();
        let palette = ancestors
            .iter()
            .find_map(
                |n| match (n.tag_name().name(), node_name(*n).map(str::trim)) {
                    (
                        "VASSAL.build.module.ChartWindow",
                        Some("Organisation Charts (with units)"),
                    ) => Some("organisation_charts"),
                    ("VASSAL.build.module.PieceWindow", Some("Units")) => Some("units"),
                    ("VASSAL.build.module.PieceWindow", Some("Markers")) => Some("markers"),
                    ("VASSAL.build.module.Map", Some("Main Map")) => Some("main_map"),
                    _ => None,
                },
            )
            .ok_or_else(|| format!("GPID {gpid}: unrecognized palette"))?;
        let palette_path = ancestors
            .iter()
            .rev()
            .filter_map(|n| node_name(*n).map(|name| format!("{}:{name}", n.tag_name().name())))
            .collect();
        let prefix = entry_name.split_whitespace().next().unwrap_or("");
        let side = match prefix {
            "BR" | "IN" | "AU" | "NZ" | "SA" | "AL" => Some("allied"),
            "IT" | "GE" | "AX" => Some("axis"),
            _ => None,
        };
        let mut slot = Slot {
            gpid,
            nationality_prefix: side.map(|_| prefix.to_owned()),
            side: side.map(str::to_owned),
            entry_name,
            source_line: doc.text_pos_at(node.range().start).row,
            palette: palette.to_owned(),
            palette_path,
            ..Slot::default()
        };
        inspect_traits(
            node.text().unwrap_or(""),
            "slot",
            &prototypes,
            &mut Vec::new(),
            images,
            &mut slot,
        )
        .map_err(|e| format!("GPID {}: {e}", slot.gpid))?;
        slot.slot_kind = if ancestors.iter().any(|n| node_name(*n) == Some("Labels")) {
            "label"
        } else if palette == "markers" || palette == "main_map" {
            "marker"
        } else if slot.prototype_refs.iter().any(|p| {
            [
                "Properties - Units",
                "Royal Navy",
                "Coastal Shipping",
                "AirPlanes",
            ]
            .contains(&p.as_str())
        }) {
            "unit_counter"
        } else {
            "chart_decoration"
        }
        .to_owned();
        slots.push(slot);
    }
    // Palette-family links are deliberately weaker than canonical identity.
    // Equal labels alone, same shared border, or same-palette rows never link.
    let mut oc: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for slot in slots
        .iter()
        .filter(|s| s.palette == "organisation_charts" && s.slot_kind == "unit_counter")
    {
        if let Some(stem) = Path::new(&slot.basic_image)
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
        {
            oc.entry((slot.entry_name.clone(), stem.to_owned()))
                .or_default()
                .push(slot.gpid.clone());
        }
    }
    for slot in slots
        .iter_mut()
        .filter(|s| s.palette == "units" && s.slot_kind == "unit_counter")
    {
        if let Some(stem) = Path::new(&slot.basic_image)
            .file_stem()
            .and_then(|s| s.to_str())
        {
            slot.duplicate_palette_candidates = oc
                .get(&(slot.entry_name.clone(), stem.to_owned()))
                .cloned()
                .unwrap_or_default();
            if !slot.duplicate_palette_candidates.is_empty() {
                slot.slot_kind = "duplicate_palette_entry".to_owned();
            }
        }
    }
    let mut summary = Summary {
        slots: slots.len(),
        distinct_entry_names: slots
            .iter()
            .map(|s| &s.entry_name)
            .collect::<BTreeSet<_>>()
            .len(),
        ..Summary::default()
    };
    for slot in &slots {
        *summary.palettes.entry(slot.palette.clone()).or_default() += 1;
        *summary.kinds.entry(slot.slot_kind.clone()).or_default() += 1;
        summary.missing_images.extend(
            slot.images
                .iter()
                .filter(|i| !i.exists)
                .map(|i| i.name.clone()),
        );
    }
    Ok(Census {
        schema_version: 1,
        interpretation: "Source slots, not historical identities. Palette-copy candidates require equal entry labels and base-image stems across OC/Units palettes; they never merge GPIDs. Side/prefix values describe source labels, not adjudicated nationalities. Missing images remain listed. No saves read.",
        summary, slots,
    })
}
