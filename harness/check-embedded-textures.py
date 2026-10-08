#!/usr/bin/env python3
"""Check public embedded texture completeness/hashes; --pixels also requires Pillow."""
# SPDX-License-Identifier: GPL-3.0-or-later
import hashlib
import json
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / "assets/neverball/EMBEDDED-TEXTURES.json").read_text())
files = manifest["files"]
required = {"assets/neverball/" + p for p in re.findall(r'tex!\(\s*"([^"]+)"', (root / "src/visual.rs").read_text())}
assert len(files) == 20
assert required == {f["path"] for f in files}, "embedded texture inventory differs from renderer"
for entry in files:
    target = root / entry["path"]
    source = root / entry["source"]
    assert hashlib.sha256(target.read_bytes()).hexdigest() == entry["sha256"], entry["path"]
    assert hashlib.sha256(source.read_bytes()).hexdigest() == entry["source_sha256"], entry["source"]
    assert target.stat().st_size == entry["bytes"]
    if source.suffix == ".png":
        assert target.read_bytes() == source.read_bytes(), entry["path"]
    if "--pixels" in sys.argv:
        from PIL import Image
        a = Image.open(source).convert("RGB")
        b = Image.open(target).convert("RGB")
        assert a.size == b.size == tuple(entry["dimensions"])
        assert a.tobytes() == b.tobytes(), entry["path"]
        assert hashlib.sha256(a.tobytes()).hexdigest() == entry["decoded_rgb_sha256"]
print("20 embedded textures: complete inventory and hashes; 12 original PNG copies, 8 JPEG-derived PNGs" + ("; all decoded pixels match" if "--pixels" in sys.argv else ""))
