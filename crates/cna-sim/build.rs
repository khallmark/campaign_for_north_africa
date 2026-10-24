use std::{fs, path::Path};

// Deliberately conservative, grep-style boundary lint, not a Rust name resolver.
// Also checks unused source files so a future module declaration cannot enable
// a previously unchecked direct dependency on demonstration rules.
fn check_official(path: &Path) {
    for entry in fs::read_dir(path).expect("read official source directory") {
        let path = entry.expect("read official source entry").path();
        if path.is_dir() {
            check_official(&path);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let source = fs::read_to_string(&path).expect("read official Rust source");
            assert!(
                !source
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|token| token == "sandbox"),
                "{}: official source must not reference sandbox; use official types and rules",
                path.display()
            );
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=src/official");
    let official = Path::new("src/official");
    if official.exists() {
        check_official(official);
    }
}
