#!/usr/bin/env python3
"""Validate exported assets and their application copies; write a reviewable report."""

import hashlib
import io
import json
import plistlib
import re
import struct
import tomllib
import xml.etree.ElementTree as ET
from html.parser import HTMLParser
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
report = {"png": [], "svg": [], "icns": [], "integration": [], "checks": []}


def check(condition, message):
    if not condition:
        raise ValueError(message)


exports = json.loads((ROOT / "manifest.json").read_text())["files"]
for item in exports:
    data = (ROOT / item["file"]).read_bytes()
    check(hashlib.sha256(data).hexdigest() == item["sha256"], f"Stale export: {item['file']}")

for file in sorted(ROOT / item["file"] for item in exports if item["file"].endswith(".svg")):
    doc = ET.parse(file).getroot()
    check(doc.tag == "{http://www.w3.org/2000/svg}svg", f"Invalid SVG: {file}")
    check("viewBox" in doc.attrib, f"Missing viewBox: {file}")
    tags = {element.tag.split("}")[-1] for element in doc.iter()}
    check(not tags & {"image", "text", "filter"}, f"Non-portable content in {file}")
    report["svg"].append({"file": str(file.relative_to(ROOT)), "viewBox": doc.attrib["viewBox"]})

for file in sorted(ROOT / item["file"] for item in exports if item["file"].endswith(".png")):
    with Image.open(file) as image:
        image.load()
        check(image.format == "PNG" and image.mode == "RGBA", f"Expected RGBA PNG: {file}")
        alpha = image.getchannel("A")
        if file.name == "apple-touch-icon.png":
            check(image.size == (180, 180) and alpha.getextrema() == (255, 255), "Touch icon must be opaque 180px")
        else:
            check(alpha.getextrema() == (0, 255), f"Missing transparency: {file}")
            check(alpha.getpixel((0, 0)) == 0, f"Opaque outer corner: {file}")
        if file.name.startswith("unwrap-app-icon-"):
            check(image.size == (1024, 1024), f"App master not 1024px: {file}")
        size_match = re.fullmatch(r"(?:unwrap-icon|favicon)-(\d+)\.png", file.name)
        if size_match:
            size = int(size_match[1])
            check(image.size == (size, size), f"Incorrect dimensions: {file}")
        report["png"].append({"file": str(file.relative_to(ROOT)), "size": list(image.size), "mode": image.mode})

data = (ROOT / "unwrap.icns").read_bytes()
check(data[:4] == b"icns" and struct.unpack_from(">I", data, 4)[0] == len(data), "Invalid ICNS header")
expected = {"icp4": 16, "ic11": 32, "icp5": 32, "ic12": 64, "ic07": 128, "ic13": 256, "ic08": 256, "ic14": 512, "ic09": 512, "ic10": 1024}
offset = 8
seen = set()
while offset < len(data):
    kind = data[offset:offset + 4].decode()
    length = struct.unpack_from(">I", data, offset + 4)[0]
    check(length > 8 and offset + length <= len(data), "Invalid ICNS chunk boundary")
    with Image.open(io.BytesIO(data[offset + 8:offset + length])) as image:
        image.load()
        check(image.size == (expected[kind], expected[kind]), f"Incorrect ICNS representation: {kind}")
        report["icns"].append({"type": kind, "pixels": image.width})
    seen.add(kind)
    offset += length
check(seen == set(expected), "Incomplete ICNS representations")
with Image.open(ROOT / "unwrap.icns") as image:
    image.load()
    check(image.size == (1024, 1024), "ICNS decoder did not load master")
with Image.open(ROOT / "unwrap.ico") as image:
    check(image.ico.sizes() == {(s, s) for s in [16, 24, 32, 48, 64, 128, 256]}, "Incomplete ICO")
    for size in image.ico.sizes():
        image.ico.getimage(size).load()
report["checks"].append("ICNS: ten native representations; ICO: seven decoded representations")

for size in [16, 32]:
    with Image.open(ROOT / f"favicon-{size}.png") as image:
        alpha = image.getchannel("A")
        check(alpha.getpixel((size // 2, size // 2)) == 0, "Favicon opening closed")
        check(alpha.getpixel((size // 2, int(size * .8))) > 200, "Favicon base missing")
        check(alpha.getpixel((int(size * .25), size // 2)) > 200, "Favicon left arm missing")
        check(alpha.getpixel((int(size * .75), size // 2)) > 200, "Favicon right arm missing")
report["checks"].append("16px and 32px: open center, visible arms and unified base")


def luminance(hex_color):
    values = [int(hex_color[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    linear = [v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4 for v in values]
    return sum(v * weight for v, weight in zip(linear, [0.2126, 0.7152, 0.0722]))


report["contrast"] = []
for background in ["#0B0D0E", "#F5F5F5"]:
    low, high = sorted([luminance("#0D9F5F"), luminance(background)])
    ratio = (high + 0.05) / (low + 0.05)
    check(ratio >= 3, "Small-icon solid fill needs at least 3:1 contrast")
    report["contrast"].append({"foreground": "#0D9F5F", "background": background, "ratio": round(ratio, 2)})

mapping = json.loads((ROOT / "integration.json").read_text())
for source, destination in mapping.items():
    check((ROOT / source).read_bytes() == (REPO / destination).read_bytes(), f"Stale application asset: {destination}")
    report["integration"].append(destination)
config = tomllib.loads((REPO / "app/Cargo.toml").read_text())["package"]["metadata"]["bundle"]["bin"]["warp-oss"]
check(config["name"] == "Unwrap", "Incorrect OSS bundle name")
check(config["identifier"] == "dev.warp.WarpOss", "Application data identity changed")
for file in config["icon"]:
    check((REPO / "app" / file).is_file(), f"Missing packaging icon: {file}")
source = (REPO / "app/src/bin/oss.rs").read_text()
plist = plistlib.loads(re.search(r'<\?xml.*?</plist>', source, re.S).group().encode())
check(plist["CFBundleName"] == plist["CFBundleDisplayName"] == "Unwrap", "Incorrect embedded display name")
report["checks"].append("Packaging references exist; OSS display names agree; persistent app ID preserved")


class PreviewParser(HTMLParser):
    def handle_starttag(self, tag, attrs):
        for key, value in attrs:
            if key in ("href", "src") and value and value != "validation.json" and not value.startswith(("#", "http")):
                check((ROOT / value).exists(), f"Broken preview reference: {value}")


PreviewParser().feed((ROOT / "preview.html").read_text())
report["checks"].append("Preview references resolve to actual exported files")
report["status"] = "passed"
(ROOT / "validation.json").write_text(json.dumps(report, indent=2) + "\n")
print(f"Validated {len(report['png'])} PNGs, {len(report['svg'])} SVGs, ICNS, ICO and {len(mapping)} application copies")
