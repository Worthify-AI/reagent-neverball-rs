# Browser menu metadata

The browser loads `data/level-metadata-index.json` for its menu catalogue. This contains the eight set manifests and every existing `LevelSpec::from_sol` field for their 183 courses, in manifest order. It avoids fetching every course just to build the menu catalogue. The existing launch path loads the selected course geometry on demand. Native menus continue parsing the SOL files directly.

Regenerate from the repository root with `python3 harness/generate-level-metadata.py`. This runs the public Rust generator using this reconstruction's SOL parser and records SHA-256 hashes of every set/SOL input, the parser/generator, and the index in `data/level-metadata-provenance.json`. Output is deterministic and contains no timestamps or private workspace paths. `build-web.sh` regenerates it before building and records both JSON hashes in the loader manifest.

`cargo test --locked --test level_metadata` compares every catalogue field against fresh parsing of all packaged course files, checks deterministic serialization, and rejects incompatible or misaligned indexes. Metadata is derived from licensed runtime data, with the same attribution and terms described in `NOTICE.md` and `THIRD_PARTY_NOTICES.md`. No original engine implementation source is used.
