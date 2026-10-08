#!/usr/bin/env python3
"""Regenerate menu metadata with our Rust parser and record exact runtime inputs."""
# SPDX-License-Identifier: GPL-3.0-or-later
import hashlib
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
subprocess.run(["cargo", "run", "--locked", "--bin", "generate-level-metadata", "--", "data"], cwd=root, check=True)
data = root / "data"
index = json.loads((data / "level-metadata-index.json").read_text())
paths = {"sets.txt"}
for level_set in index["catalog"]["sets"]:
    paths.add(level_set["file"])
    paths.update(level_set["levels"])
hash_file = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
manifest = {
    "format": 1,
    "method": "Our Rust Catalog::parse, Sol::from_bytes and LevelSpec::from_sol applied to packaged runtime data; no original engine implementation source.",
    "generator": "harness/generate-level-metadata.py",
    "index_sha256": hash_file(data / "level-metadata-index.json"),
    "set_count": len(index["catalog"]["sets"]),
    "level_count": sum(len(s["levels"]) for s in index["catalog"]["sets"]),
    "source_sha256": {path: hash_file(data / path) for path in sorted(paths)},
    "parser_sha256": {path: hash_file(root / path) for path in ["src/content.rs", "src/sol.rs", "src/bin/generate-level-metadata.rs"]},
}
(data / "level-metadata-provenance.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Generated {manifest['level_count']} levels from {manifest['set_count']} sets; index SHA-256 {manifest['index_sha256']}")
