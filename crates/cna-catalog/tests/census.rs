use cna_catalog::census::scan;
use std::path::Path;

fn module() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/CNAv2.1.0")
}

#[test]
fn census_accounts_for_every_source_slot_without_merging() {
    let xml = std::fs::read_to_string(module().join("buildFile.xml")).unwrap();
    let census = scan(&xml, &module().join("images")).unwrap();
    let source = roxmltree::Document::parse(&xml).unwrap();
    let slots: Vec<_> = source
        .descendants()
        .filter(|n| n.has_tag_name("VASSAL.build.widget.PieceSlot"))
        .collect();
    assert_eq!(slots.len(), 2468);
    assert_eq!(census.slots.len(), slots.len());
    for (row, node) in census.slots.iter().zip(slots) {
        assert_eq!(row.gpid, node.attribute("gpid").unwrap());
        assert_eq!(row.entry_name, node.attribute("entryName").unwrap());
    }
    assert_eq!(census.summary.distinct_entry_names, 1307);
    assert_eq!(census.summary.palettes["organisation_charts"], 1194);
    assert_eq!(census.summary.palettes["units"], 1164);
    assert_eq!(census.summary.palettes["markers"], 102);
    assert_eq!(census.summary.palettes["main_map"], 8);

    let weather = census.slots.iter().find(|s| s.gpid == "6497").unwrap();
    let layer = weather.layers.iter().find(|l| l.name == "Weather").unwrap();
    assert_eq!(layer.levels.len(), 4);
    assert_eq!(layer.levels[2].image, "Rain.svg");
    assert_eq!(layer.levels[2].label, "Rain Storm");
    let unit = census.slots.iter().find(|s| s.gpid == "3056").unwrap();
    assert_eq!(unit.nationality_prefix.as_deref(), Some("SA"));
    assert_eq!(unit.side.as_deref(), Some("allied"));
    assert!(unit
        .images
        .iter()
        .any(|i| i.name == "moved flag.svg" && i.exists));
    assert!(unit.images.iter().any(|i| i.name == "SA UMtRF - 4-2SA.svg"));
}

#[test]
fn census_preserves_missing_images_and_only_links_evidenced_palette_copies() {
    // Literal VASSAL nesting: slash command, tab decorators, semicolon fields.
    // Three equal labels must not collapse two different image identities.
    let xml = r#"<VASSAL.build.GameModule>
      <VASSAL.build.module.PrototypesContainer>
        <VASSAL.build.module.PrototypeDefinition name="Properties - Units">+/null/markmoved;missing-flag.svg;0;0&#9;piece;;;;/false&#9;null;0;0;;0</VASSAL.build.module.PrototypeDefinition>
      </VASSAL.build.module.PrototypesContainer>
      <VASSAL.build.module.ChartWindow name="Organisation Charts (with units)">
        <VASSAL.build.widget.PieceSlot gpid="1" entryName="BR Same">+/null/prototype;Properties - Units&#9;piece;;;missing.svg;BR Same/&#9;null;0;0;1;0</VASSAL.build.widget.PieceSlot>
      </VASSAL.build.module.ChartWindow>
      <VASSAL.build.module.PieceWindow name="Units ">
        <VASSAL.build.widget.PieceSlot gpid="2" entryName="BR Same">+/null/prototype;Properties - Units&#9;piece;;;missing.png;BR Same/&#9;null;0;0;2;0</VASSAL.build.widget.PieceSlot>
        <VASSAL.build.widget.PieceSlot gpid="3" entryName="BR Same">+/null/prototype;Properties - Units&#9;piece;;;other.svg;BR Same/&#9;null;0;0;3;0</VASSAL.build.widget.PieceSlot>
      </VASSAL.build.module.PieceWindow>
    </VASSAL.build.GameModule>"#;
    let census = scan(xml, &module().join("images")).unwrap();
    assert_eq!(census.slots.len(), 3);
    assert_eq!(census.slots[1].duplicate_palette_candidates, ["1"]);
    assert_eq!(census.slots[1].slot_kind, "duplicate_palette_entry");
    assert!(census.slots[2].duplicate_palette_candidates.is_empty());
    assert_eq!(census.slots[2].slot_kind, "unit_counter");
    assert!(census.slots[0]
        .images
        .iter()
        .any(|i| i.name == "missing.svg" && !i.exists));
    assert!(census.slots[0]
        .images
        .iter()
        .any(|i| i.name == "missing-flag.svg" && !i.exists));
    assert!(census.summary.missing_images.contains("missing-flag.svg"));
    let cycle = xml.replace(
        "markmoved;missing-flag.svg;0;0",
        "prototype;Properties - Units",
    );
    assert!(scan(&cycle, &module().join("images"))
        .unwrap_err()
        .contains("cycle"));
    let duplicate = xml.replace("gpid=\"3\"", "gpid=\"1\"");
    assert!(scan(&duplicate, &module().join("images"))
        .unwrap_err()
        .contains("duplicate GPID"));
}

#[test]
fn census_cli_check_verifies_checked_in_output_without_writing() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vassal-census"))
        .args(["--check"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("2468 slots"));
}
