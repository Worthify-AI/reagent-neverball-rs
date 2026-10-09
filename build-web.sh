#!/bin/sh
set -eu
repo=$(pwd)
python3 harness/generate-level-metadata.py
# Serve the retained runtime data without duplicating it in repository history.
[ -e web/data ] || ln -s ../data web/data
export RUSTFLAGS="-C link-arg=--import-undefined --remap-path-prefix=$repo=. --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=cargo --remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=rust"
cargo build --locked --release --target wasm32-unknown-unknown --bin reagent-neverball-rs
cp target/wasm32-unknown-unknown/release/reagent-neverball-rs.wasm web/reagent_neverball_rs.wasm
python3 - <<'PYHASH'
from pathlib import Path
import hashlib, json
web = Path("web")
path = web / "loader-manifest.json"
manifest = json.loads(path.read_text())
manifest["level_metadata_index_sha256"] = hashlib.sha256(Path("data/level-metadata-index.json").read_bytes()).hexdigest()
manifest["level_metadata_provenance_sha256"] = hashlib.sha256(Path("data/level-metadata-provenance.json").read_bytes()).hexdigest()
manifest["wasm_sha256"] = hashlib.sha256((web / "reagent_neverball_rs.wasm").read_bytes()).hexdigest()
manifest["loader_sha256"] = hashlib.sha256((web / "gl.js").read_bytes()).hexdigest()
manifest["bridge_files_sha256"] = {f:hashlib.sha256((web / f).read_bytes()).hexdigest() for f in ["boot.js","save-storage.js","input-controls.js","tilt-controls.js"]}
manifest["runtime_assets_manifest_sha256"] = hashlib.sha256(Path("data/ASSET-MANIFEST.json").read_bytes()).hexdigest()
manifest["runtime_artwork_provenance_sha256"] = hashlib.sha256(Path("data/runtime-artwork/PROVENANCE.json").read_bytes()).hexdigest()
manifest["preferred_form_manifest_sha256"] = hashlib.sha256(Path("assets/neverball/source-full/SOURCE-ASSETS.json").read_bytes()).hexdigest()
manifest["audio_loader"] = {"crate":"quad-snd","version":"0.2.8","file":"quad-snd.js","sha256":hashlib.sha256((web / "quad-snd.js").read_bytes()).hexdigest(),"modifications":["Handle decode rejection and complete failed cues with silence"]}
manifest["browser_audio_provenance_sha256"] = hashlib.sha256(Path("data/web-audio/PROVENANCE.json").read_bytes()).hexdigest()
path.write_text(json.dumps(manifest, indent=2) + "\n")
PYHASH
