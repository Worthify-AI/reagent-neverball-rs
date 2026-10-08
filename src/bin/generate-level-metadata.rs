// SPDX-License-Identifier: GPL-3.0-or-later
//! Generate the browser menu catalogue using our SOL parser; no reference source needed.
use reagent_neverball_rs::content::{Catalog, LevelMetadataIndex};
use std::{fs, path::PathBuf};
fn main() -> Result<(), String> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "data".into()));
    let read = |path: &str| fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"));
    let catalog = Catalog::parse(&read("sets.txt")?, read)?;
    let index = LevelMetadataIndex::generate(catalog, |path| {
        fs::read(root.join(path)).map_err(|e| format!("{path}: {e}"))
    })?;
    let mut json = serde_json::to_string(&index).map_err(|e| e.to_string())?;
    json.push('\n');
    fs::write(root.join("level-metadata-index.json"), json).map_err(|e| e.to_string())
}
