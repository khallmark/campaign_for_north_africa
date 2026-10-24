use cna_catalog::census::scan;
use std::{env, fs, path::PathBuf, process::ExitCode};

const HELP: &str = "vassal-census — inventory buildFile.xml without merging identities or reading saves

Usage: vassal-census [--module PATH] [--output PATH] [--check]
  --module PATH  buildFile.xml (default: docs/CNAv2.1.0/buildFile.xml)
                 Image references are checked in its sibling images/ directory.
  --output PATH  Output JSON (default: data/sources/vassal-2.1-pieces.json)
  --check        Compare with existing output; exit 1 on drift, never write.
  -h, --help     Show this help. Unknown/missing arguments exit 1.

Examples (from repository root):
  cargo run -p cna-catalog --bin vassal-census
  cargo run -p cna-catalog --bin vassal-census -- --check
  cargo run -p cna-catalog --bin vassal-census -- --module /tmp/module/buildFile.xml --output /tmp/pieces.json";

fn run() -> Result<(), String> {
    let mut module = PathBuf::from("docs/CNAv2.1.0/buildFile.xml");
    let mut output = PathBuf::from("data/sources/vassal-2.1-pieces.json");
    let mut check = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{HELP}");
                return Ok(());
            }
            "--check" => check = true,
            "--module" | "--output" => {
                let value = args
                    .next()
                    .filter(|s| !s.starts_with('-'))
                    .ok_or_else(|| format!("{arg} requires a path; use --help for examples"))?;
                if arg == "--module" {
                    module = value.into();
                } else {
                    output = value.into();
                }
            }
            _ => return Err(format!("unknown argument {arg:?}; use --help for examples")),
        }
    }
    if module.extension().is_none_or(|e| e != "xml") {
        return Err("--module must be a buildFile.xml, not a save file".to_owned());
    }
    let xml = fs::read_to_string(&module).map_err(|e| format!("{}: {e}", module.display()))?;
    let images = module.parent().unwrap().join("images");
    if !images.is_dir() {
        return Err(format!(
            "{} is missing; restore the module images before generating a census",
            images.display()
        ));
    }
    let census = scan(&xml, &images)?;
    let mut json = serde_json::to_string_pretty(&census).map_err(|e| e.to_string())?;
    json.push('\n');
    if check {
        let existing = fs::read_to_string(&output).map_err(|e| {
            format!(
                "{}: {e}; run without --check to generate it",
                output.display()
            )
        })?;
        if existing != json {
            return Err(format!(
                "{} is stale; regenerate without --check and review the diff",
                output.display()
            ));
        }
    } else {
        if output == module {
            return Err("output must not overwrite the source XML".to_owned());
        }
        if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&output, json).map_err(|e| format!("{}: {e}", output.display()))?;
    }
    println!(
        "{}: {} slots, {} distinct entry names, {} missing image references; {}",
        output.display(),
        census.summary.slots,
        census.summary.distinct_entry_names,
        census.summary.missing_images.len(),
        if check { "up to date" } else { "written" }
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vassal-census: {error}");
            ExitCode::FAILURE
        }
    }
}
