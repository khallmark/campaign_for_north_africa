#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --locked --release --target wasm32-unknown-unknown -p cna-frontend
wasm-bindgen --target web --out-dir public/wasm --out-name cna target/wasm32-unknown-unknown/release/cna_frontend.wasm
printf 'import init from "./cna.js";\nawait init();\n' > public/wasm/boot.js
