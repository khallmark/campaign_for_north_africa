use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!("cna-map-tiles — tile generation is not yet implemented (map E2.I1.S1).\n\nExamples:\n  cargo run --release -p cna-map --bin cna-map-tiles -- --help");
        return ExitCode::SUCCESS;
    }
    eprintln!("cna-map-tiles: tile generation is not yet implemented; blocked on map E2.I1.S1. No tiles were written.");
    ExitCode::FAILURE
}
