#!/usr/bin/env python3
"""Validate the app asset integration separately from the master brand exports."""

import json
import re
import xml.etree.ElementTree as ET
from pathlib import Path

from PIL import Image

BRAND = Path(__file__).resolve().parents[1]
ROOT = BRAND.parents[1]
SVG = "{http://www.w3.org/2000/svg}"
master = ET.parse(BRAND / "unwrap-symbol-white.svg").getroot()
silhouette = master.find(f"{SVG}path").attrib["d"]
outputs = json.loads((BRAND / "app-integration.json").read_text())["outputs"]
assert len(outputs) == len(set(outputs)), "Duplicate app asset mappings"
report = []

for relative in outputs:
    path = ROOT / relative
    assert path.is_file(), f"Missing app asset: {relative}"
    record = {"file": relative, "bytes": path.stat().st_size}
    if path.suffix == ".svg":
        doc = ET.parse(path).getroot()
        assert doc.tag == f"{SVG}svg" and "viewBox" in doc.attrib, relative
        assert not any(node.tag in {f"{SVG}image", f"{SVG}text", f"{SVG}filter"} for node in doc.iter()), relative
        if path.name.startswith("warp-loading-") or path.name == "unwrap-symbol.svg":
            assert doc.find(f"{SVG}path").attrib["d"] == silhouette, relative
        if path.name.startswith("warp-loading-"):
            opacity = float(doc.find(f"{SVG}path").attrib["opacity"])
            assert 0.5 <= opacity <= 1, relative
            record["opacity"] = opacity
    elif path.suffix in {".png", ".bmp", ".ico"}:
        with Image.open(path) as image:
            image.load()
            record["format"] = image.format
            record["size"] = list(image.size)
            size_match = re.fullmatch(r"(\d+)x\1.png", path.name)
            if size_match:
                size = int(size_match[1])
                assert image.size == (size, size), relative
                assert image.mode == "RGBA", relative
            if "DockTilePlugin/Resources/" in relative:
                assert image.size == (512, 512), relative
                assert image.getchannel("A").getpixel((0, 0)) == 0, relative
            if path.name == "warp_install_image.png":
                assert image.size == (700, 500), relative
            if "warpdrive" in path.name:
                assert image.size == (1803, 2040), relative
            if path.name == "warp-logo.bmp":
                assert image.size == (58, 58), relative
            if path.name == "warp-banner.bmp":
                assert image.size == (202, 385), relative
            if path.suffix == ".ico":
                assert image.ico.sizes() == {(n, n) for n in [16, 24, 32, 48, 64, 128, 256]}, relative
    elif path.name == "icon.json":
        config = json.loads(path.read_text())
        for group in config["groups"]:
            for layer in group["layers"]:
                assert (path.parent / "Assets" / layer["image-name"]).is_file(), relative
    report.append(record)

assert len([r for r in report if "opacity" in r]) == 12
for relative in [
    "app/src/ai/loading/shimmering_warp_loading_text.rs",
    "app/src/ai/blocklist/block/view_impl/common.rs",
]:
    assert "E500" not in (ROOT / relative).read_text(), relative

(BRAND / "app-validation.json").write_text(json.dumps({"assets": report}, indent=2) + "\n")
print(f"Validated {len(report)} app assets, all 12 loading frames, and font-independent loading symbols")
