#!/bin/sh
set -eu
repo=$(pwd)
export RUSTFLAGS="-C link-arg=--import-undefined --remap-path-prefix=$repo=. --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=cargo --remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=rust"
cargo build --locked --release --target wasm32-unknown-unknown --bin reagent-neverball-rs
cp target/wasm32-unknown-unknown/release/reagent-neverball-rs.wasm web/reagent_neverball_rs.wasm
