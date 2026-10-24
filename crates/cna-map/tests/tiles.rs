use image::{Rgba, RgbaImage};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

fn run(source: &Path, output: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cna-map-tiles"))
        .arg("--source")
        .arg(source)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

#[test]
fn tiles_are_deterministic_cropped_and_incrementally_verified() {
    let root = std::env::temp_dir().join(format!("cna-tiles-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let source = root.join("source.png");
    let input = RgbaImage::from_fn(515, 519, |x, y| {
        Rgba([
            (x % 251) as u8,
            (y % 239) as u8,
            ((x * 3 + y * 7) % 253) as u8,
            255,
        ])
    });
    input.save(&source).unwrap();
    let out = root.join("first");
    let generated = run(&source, &out);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let manifest_bytes = fs::read(out.join("manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest["board_width"], 515);
    assert_eq!(manifest["board_height"], 519);
    assert_eq!(manifest["tile_size"], 512);
    assert_eq!(
        manifest["source_sha256"],
        format!("{:x}", Sha256::digest(fs::read(&source).unwrap()))
    );
    let levels = manifest["levels"].as_array().unwrap();
    assert_eq!(levels.len(), 2);
    assert_eq!(
        (levels[0]["width"].as_u64(), levels[0]["height"].as_u64()),
        (Some(258), Some(260))
    );
    let top = &levels[1];
    assert_eq!(top["downsample"], 1);
    assert_eq!(top["columns"], 2);
    assert_eq!(top["rows"], 2);
    assert_eq!(top["tiles"].as_array().unwrap().len(), 4);
    let mut area = 0;
    for tile in top["tiles"].as_array().unwrap() {
        let (x, y) = (
            tile["x"].as_u64().unwrap() as u32,
            tile["y"].as_u64().unwrap() as u32,
        );
        let path = out.join(format!("1/{x}_{y}.png"));
        let bytes = fs::read(&path).unwrap();
        assert_eq!(tile["sha256"], format!("{:x}", Sha256::digest(&bytes)));
        assert_eq!(tile["bytes"], bytes.len());
        let decoded = image::open(path).unwrap().to_rgba8();
        // Independent bounds: last column is 3px, last row 7px, never 512px padding.
        let expected = (if x == 0 { 512 } else { 3 }, if y == 0 { 512 } else { 7 });
        assert_eq!(decoded.dimensions(), expected);
        assert_eq!(tile["width"], expected.0);
        assert_eq!(tile["height"], expected.1);
        area += decoded.width() * decoded.height();
        for (px, py, color) in decoded.enumerate_pixels() {
            assert_eq!(color, input.get_pixel(x * 512 + px, y * 512 + py));
        }
    }
    assert_eq!(area, 515 * 519);
    let modified = fs::metadata(out.join("1/1_1.png"))
        .unwrap()
        .modified()
        .unwrap();
    let repeat = run(&source, &out);
    assert!(repeat.status.success());
    assert!(String::from_utf8_lossy(&repeat.stdout).contains("unchanged:"));
    assert_eq!(
        fs::metadata(out.join("1/1_1.png"))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
    assert_eq!(fs::read(out.join("manifest.json")).unwrap(), manifest_bytes);
    let fresh = root.join("second");
    assert!(run(&source, &fresh).status.success());
    assert_eq!(
        fs::read(fresh.join("manifest.json")).unwrap(),
        manifest_bytes
    );
    // A source-only incremental check would wrongly accept this corrupted tile.
    fs::write(out.join("1/1_1.png"), b"broken").unwrap();
    assert!(run(&source, &out).status.success());
    assert_eq!(
        fs::read(out.join("1/1_1.png")).unwrap(),
        fs::read(fresh.join("1/1_1.png")).unwrap()
    );
    assert_eq!(fs::read(out.join("manifest.json")).unwrap(), manifest_bytes);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tiles_cli_rejects_bad_inputs_without_overwriting_unowned_output() {
    let root = std::env::temp_dir().join(format!("cna-tiles-input-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let source = root.join("source.png");
    RgbaImage::new(2, 3).save(&source).unwrap();
    let out = root.join("occupied");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("keep.txt"), b"existing work").unwrap();
    assert!(!run(&source, &out).status.success());
    assert_eq!(fs::read(out.join("keep.txt")).unwrap(), b"existing work");
    for args in [
        vec!["--source"],
        vec!["--output", "--help"],
        vec!["--unknown"],
    ] {
        let invalid = Command::new(env!("CARGO_BIN_EXE_cna-map-tiles"))
            .args(args)
            .output()
            .unwrap();
        assert!(!invalid.status.success());
        assert!(String::from_utf8_lossy(&invalid.stderr).contains("--help"));
    }
    let help = Command::new(env!("CARGO_BIN_EXE_cna-map-tiles"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Examples:"));
    fs::remove_dir_all(root).unwrap();
}
