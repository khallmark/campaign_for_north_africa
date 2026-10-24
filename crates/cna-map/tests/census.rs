use roxmltree::Document;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn census_preserves_every_main_map_zone_and_grid_parameter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let xml = fs::read_to_string(root.join("docs/CNAv2.1.0/buildFile.xml")).unwrap();
    let doc = Document::parse(&xml).unwrap();
    let main = doc
        .descendants()
        .find(|n| n.attribute("mapName") == Some("Main Map"))
        .unwrap();
    let source_zones: Vec<_> = main
        .descendants()
        .filter(|n| n.tag_name().name().ends_with(".Zone"))
        .collect();
    let census: Value =
        serde_json::from_slice(&fs::read(root.join("public/data/map/sources.json")).unwrap())
            .unwrap();
    let zones = census["zones"].as_array().unwrap();
    assert_eq!(
        source_zones.len(),
        33,
        "review any source-zone additions/removals"
    );
    assert_eq!(zones.len(), source_zones.len());
    let hex_names: Vec<_> = zones
        .iter()
        .filter(|z| z["kind"] == "hex_section")
        .map(|z| z["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        hex_names,
        ["Malta", "Map A", "Map B", "Map C", "Map D", "Map E"]
    );
    for (record, source) in zones.iter().zip(&source_zones) {
        assert_eq!(record["name"], source.attribute("name").unwrap());
        let points: Vec<Vec<i32>> = source
            .attribute("path")
            .unwrap()
            .split(';')
            .map(|p| p.split(',').map(|c| c.parse().unwrap()).collect())
            .collect();
        assert_eq!(record["polygon"], json!(points));
        let source_grid = source.children().find(|n| n.is_element());
        if let Some(grid) = source_grid {
            let attrs: BTreeMap<_, _> = grid.attributes().map(|a| (a.name(), a.value())).collect();
            assert_eq!(record["grid"]["attributes"], json!(attrs));
            let kind = if grid.tag_name().name().ends_with(".HexGrid") {
                "hex"
            } else {
                "square"
            };
            assert_eq!(record["grid"]["kind"], kind);
            if let Some(numbering) = grid.children().find(|n| n.is_element()) {
                let attrs: BTreeMap<_, _> = numbering
                    .attributes()
                    .map(|a| (a.name(), a.value()))
                    .collect();
                assert_eq!(record["grid"]["numbering"], json!(attrs));
            } else {
                assert!(record["grid"]["numbering"].is_null());
            }
        } else {
            assert!(record["grid"].is_null());
        }
    }
    for (kind, count) in [("off_map_box", 14), ("track", 4), ("ui_track", 9)] {
        assert_eq!(zones.iter().filter(|z| z["kind"] == kind).count(), count);
    }
    // Asymmetric offsets and the original doubled space must survive serialization.
    assert_eq!(zones[1]["grid"]["numbering"]["stagger"], "true");
    assert_eq!(zones[5]["grid"]["numbering"]["vOff"], "-133");
    assert!(zones
        .iter()
        .any(|z| z["name"] == "Alexandria  Holding Box Hex 3714"));
    assert_eq!(census["image"]["width"], 14310);
    assert_eq!(census["image"]["height"], 4632);
    // Independently measured with sha256sum, not derived from the census implementation.
    assert_eq!(
        census["image"]["sha256"],
        "e82f995e213da486004d006e5b7fd1f6e49e7464fb7fd2db23f36bbbc57b14b4"
    );
    assert_eq!(zones[0]["hex_cell_estimate"], 71);
}

#[test]
fn census_cli_checks_current_sources_without_writing_and_rejects_unknown_options() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("public/data/map/sources.json");
    let before = fs::read(&path).unwrap();
    let check = Command::new(env!("CARGO_BIN_EXE_map-census"))
        .arg("--check")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(String::from_utf8_lossy(&check.stdout).contains("6 hex grids, 33 zones"));
    assert_eq!(fs::read(&path).unwrap(), before);
    let invalid = Command::new(env!("CARGO_BIN_EXE_map-census"))
        .arg("--chek")
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("--help"));
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn census_release_guard_rejects_pending_tiles_but_allows_explicit_local_research() {
    let bundle = std::env::temp_dir().join(format!("cna-map-release-{}", std::process::id()));
    fs::create_dir(&bundle).unwrap();
    let check = |local: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_map-census"));
        command
            .arg("--release-check")
            .arg("--bundle-dir")
            .arg(&bundle);
        if local {
            command.arg("--local-only");
        }
        command.output().unwrap()
    };
    assert!(check(false).status.success());
    fs::create_dir_all(bundle.join("tiles/map/0")).unwrap();
    assert!(
        check(false).status.success(),
        "empty directories contain no art"
    );
    fs::write(bundle.join("tiles/map/0/probe.png"), b"not real artwork").unwrap();
    let blocked = check(false);
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("not cleared for distribution"));
    let local = check(true);
    assert!(local.status.success());
    assert!(String::from_utf8_lossy(&local.stdout).contains("NOT permission to publish"));
    assert!(
        !check(false).status.success(),
        "local override must not change policy"
    );
    fs::remove_dir_all(bundle).unwrap();
}
