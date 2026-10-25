//! Machine-readable geometry export for the SQLite importer. No artwork is emitted.
use cna_map::{
    geometry::{center, corners, edge, hexes},
    hex::{Direction, HexsideId},
};
use serde_json::json;
use std::{collections::BTreeSet, error::Error, io};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!("Export the complete center-in-section geometry as JSON to stdout.\n\nExample: cargo run --locked -p cna-map --bin map-model > /tmp/map-model.json\nSQLite: python3 scripts/import-map-db.py --write");
        return Ok(());
    }
    if !args.is_empty() {
        return Err("unexpected arguments; use --help".into());
    }
    let members: BTreeSet<_> = hexes().iter().copied().collect();
    let cells: Vec<_> = hexes()
        .iter()
        .map(|&hex| {
            let c = center(hex);
            json!({"id":hex,"section":hex.section().to_string(),"col":hex.col(),"row":hex.row(),
            "center":[c.x,c.y],"corners":corners(hex).map(|p|[p.x,p.y])})
        })
        .collect();
    let mut sides = BTreeSet::new();
    for &hex in hexes() {
        for direction in Direction::ALL {
            if hex
                .neighbor(direction)
                .is_some_and(|n| members.contains(&n))
            {
                sides.insert(HexsideId::new(hex, direction).unwrap());
            }
        }
    }
    let sides: Vec<_> = sides
        .into_iter()
        .map(|side| {
            let (a, b) = edge(side);
            json!({"hex":side.hex(),"direction":side.direction(),
            "other":side.hex().neighbor(side.direction()).unwrap(),"edge":[[a.x,a.y],[b.x,b.y]]})
        })
        .collect();
    let model = cna_map::model::model();
    let surveys: Vec<_> = model
        .surveys
        .iter()
        .map(|survey| {
            json!({
                "id":survey.id,"label":survey.label,"notes":survey.notes,
                "records":survey.records()
            })
        })
        .collect();
    serde_json::to_writer(
        io::stdout().lock(),
        &json!({
            "version":1,"membership":"calibrated_center_in_source_section_polygon",
            "hexes":cells,"hexsides":sides,"sources":model.sources,"surveys":surveys
        }),
    )?;
    Ok(())
}
