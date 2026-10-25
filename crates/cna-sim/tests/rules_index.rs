use serde_json::Value;
use std::{fs, path::Path, process::Command};

#[test]
fn source_index_preserves_labels_pages_and_rejects_stale_coverage() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_rules-ledger"))
        .arg("--check-index")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(root.join("data/rules/case-index.json")).unwrap();
    let rows: Vec<Value> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(rows.len(), 1666);
    for (book, expected) in [("L21", 983), ("AL21", 593), ("SC21", 90)] {
        assert_eq!(rows.iter().filter(|r| r["book"] == book).count(), expected);
    }
    for (label, book, page, raw) in [
        ("3.6", "L21", 7, None),
        ("4.43a", "L21", 10, Some("[4.43a)]")),
        ("4.44b", "L21", 10, None),
        ("34.53", "AL21", 5, Some("[31.53]")),
        ("59.0", "SC21", 2, None),
        ("61.7", "SC21", 7, None),
        ("62.31", "SC21", 8, None),
        ("63.35", "SC21", 11, Some("(63.35)")),
    ] {
        let matches: Vec<_> = rows.iter().filter(|r| r["case"] == label).collect();
        assert_eq!(matches.len(), 1, "{label}");
        assert_eq!(matches[0]["book"], book);
        assert_eq!(matches[0]["page"], page);
        assert_eq!(matches[0]["raw_label"].as_str(), raw);
    }
    assert!(rows.iter().all(|r| r["case"] != "31.53"));
    assert!(rows.iter().all(|r| r["section"].as_u64().unwrap() < 64));

    let dir = std::env::temp_dir().join(format!("cna-index-contract-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let path = dir.join("case-index.json");
    fs::write(&path, &bytes).unwrap();
    let check = || {
        Command::new(env!("CARGO_BIN_EXE_rules-ledger"))
            .arg("--check-index")
            .arg("--data-dir")
            .arg(&dir)
            .output()
            .unwrap()
    };
    assert!(check().status.success());
    let mut missing = rows.clone();
    missing.retain(|r| r["case"] != "61.7");
    let mut wrong_page = rows.clone();
    wrong_page.iter_mut().find(|r| r["case"] == "59.0").unwrap()["page"] = 3.into();
    let mut duplicate = rows.clone();
    duplicate.push(rows[0].clone());
    for bad in [missing, wrong_page, duplicate] {
        let altered = serde_json::to_string_pretty(&bad).unwrap() + "\n";
        fs::write(&path, &altered).unwrap();
        let output = check();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("differs from source-derived output")
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            altered,
            "checking must not repair input"
        );
    }
    fs::remove_dir_all(dir).unwrap();
}
