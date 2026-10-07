#!/bin/sh
set -eu
repo=$(pwd)
export RUSTFLAGS="-C link-arg=--import-undefined --remap-path-prefix=$repo=. --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=cargo --remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=rust"
cargo build --locked --release --target wasm32-unknown-unknown --bin reagent-neverball-rs
cp target/wasm32-unknown-unknown/release/reagent-neverball-rs.wasm web/reagent_neverball_rs.wasm
python3 - <<'PYHASH'
from pathlib import Path
import hashlib, json
web = Path("web")
path = web / "loader-manifest.json"
manifest = json.loads(path.read_text())
manifest["wasm_sha256"] = hashlib.sha256((web / "reagent_neverball_rs.wasm").read_bytes()).hexdigest()
manifest["loader_sha256"] = hashlib.sha256((web / "gl.js").read_bytes()).hexdigest()
path.write_text(json.dumps(manifest, indent=2) + "\n")
PYHASH
