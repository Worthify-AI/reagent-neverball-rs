// SPDX-License-Identifier: GPL-3.0-or-later
use reagent_neverball_rs::content::{Catalog, LevelMetadataIndex};
use std::{fs, path::Path};
#[test]
fn browser_catalogue_matches_every_packaged_level_and_manifest() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let catalog = Catalog::parse(
        &fs::read_to_string(root.join("sets.txt")).unwrap(),
        |path| fs::read_to_string(root.join(path)).map_err(|e| e.to_string()),
    )
    .unwrap();
    let expected = LevelMetadataIndex::generate(catalog, |path| {
        fs::read(root.join(path)).map_err(|e| e.to_string())
    })
    .unwrap();
    let stored = fs::read_to_string(root.join("level-metadata-index.json")).unwrap();
    let actual = LevelMetadataIndex::parse(&stored).unwrap();
    assert_eq!(actual.catalog.sets.len(), 8);
    assert_eq!(actual.catalog.level_count(), 183);
    assert_eq!(actual, expected);
    assert_eq!(stored, serde_json::to_string(&expected).unwrap() + "\n");
}
#[test]
fn browser_catalogue_rejects_incompatible_or_misaligned_index() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let stored = fs::read_to_string(root.join("level-metadata-index.json")).unwrap();
    let mut index = LevelMetadataIndex::parse(&stored).unwrap();
    index.version = 2;
    assert!(LevelMetadataIndex::parse(&serde_json::to_string(&index).unwrap()).is_err());
    index.version = 1;
    index.levels[0].swap(0, 1);
    assert!(LevelMetadataIndex::parse(&serde_json::to_string(&index).unwrap()).is_err());
}
