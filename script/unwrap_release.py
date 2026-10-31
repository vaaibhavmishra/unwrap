#!/usr/bin/env python3
"""Validate Unwrap release versions, bundles, and downloadable assets."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
TAG_PATTERN = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
ARCHITECTURES = {"aarch64": "arm64", "x86_64": "x86_64"}


def version_from_tag(tag):
    if not TAG_PATTERN.fullmatch(tag):
        raise ValueError("Use a version tag such as v0.1.0 (no leading zeros or suffixes).")
    return tag[1:]


def validate_version(tag, manifest=ROOT / "app/Cargo.toml"):
    version = version_from_tag(tag)
    with manifest.open("rb") as source:
        package_version = tomllib.load(source)["package"]["version"]
    if version != package_version:
        raise ValueError(f"Tag {tag} does not match app/Cargo.toml version {package_version}.")
    return version


def asset_name(tag, arch):
    version_from_tag(tag)
    return f"Unwrap-{tag}-macos-{ARCHITECTURES[arch]}.dmg"


def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def verify_bundle(app, tag, arch):
    version = version_from_tag(tag)
    contents = app / "Contents"
    with (contents / "Info.plist").open("rb") as source:
        info = plistlib.load(source)
    expected = {
        "CFBundleName": "Unwrap",
        "CFBundleExecutable": "warp-oss",
        "CFBundleIdentifier": "dev.warp.WarpOss",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": version,
        "WarpVersion": tag,
    }
    for key, value in expected.items():
        if info.get(key) != value:
            raise ValueError(f"{key}: expected {value!r}, found {info.get(key)!r}")
    executable = contents / "MacOS/warp-oss"
    if not os.access(executable, os.X_OK):
        raise ValueError("The bundled executable is missing or is not executable.")
    slices = subprocess.check_output(["lipo", "-archs", str(executable)], text=True).split()
    if slices != [ARCHITECTURES[arch]]:
        raise ValueError(f"Expected a {ARCHITECTURES[arch]} executable, found {slices}.")
    resources = contents / "Resources"
    required_resources = (
        "THIRD_PARTY_LICENSES.txt",
        "settings_schema.json",
        "bundled/metadata/version.json",
    )
    for relative in required_resources:
        path = resources / relative
        if not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f"Missing bundled resource: {relative}")
    json.loads((resources / "settings_schema.json").read_text())
    metadata = json.loads((resources / "bundled/metadata/version.json").read_text())
    if metadata.get("warp_version") != tag:
        raise ValueError("Bundled version metadata does not match the release tag.")
    if not (contents / "PlugIns/WarpDockTilePlugin.docktileplugin").is_dir():
        raise ValueError("The dock tile plugin is missing.")


def verify_assets(tag, directory):
    expected = set()
    for arch in ARCHITECTURES:
        name = asset_name(tag, arch)
        expected.update((name, f"{name}.sha256"))
        dmg = directory / name
        checksum = directory / f"{name}.sha256"
        if not dmg.is_file() or dmg.stat().st_size == 0:
            raise ValueError(f"Missing or empty release asset: {name}")
        if checksum.read_text() != f"{sha256(dmg)}  {name}\n":
            raise ValueError(f"Checksum mismatch for {name}")
    actual = {path.name for path in directory.iterdir()}
    if actual != expected:
        raise ValueError(f"Unexpected release assets: {sorted(actual - expected)}")


def release_notes(tag, commit):
    version_from_tag(tag)
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("Release notes require a full Git commit SHA.")
    return f"""Unwrap {tag} for macOS. Built from commit `{commit}`.

Choose the **arm64** DMG for Apple Silicon or **x86_64** for Intel Macs.

These builds are unsigned and not notarized. macOS may block the first launch.
After attempting to open Unwrap, use System Settings → Privacy & Security → Open Anyway
if you trust this download. Organization-managed Macs may disallow this override.

Open the DMG and drag Unwrap.app into Applications. To update, quit Unwrap before
replacing the existing app. Existing settings remain in place. Automatic updates
are not enabled; future versions will be available on this repository's Releases page.

Each DMG has a `.sha256` file. Download both into the same directory and verify with
`shasum -a 256 -c <downloaded-dmg-name>.sha256`.

## Changes
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    version = commands.add_parser("version")
    version.add_argument("tag")
    bundle = commands.add_parser("bundle")
    bundle.add_argument("tag")
    bundle.add_argument("arch", choices=ARCHITECTURES)
    bundle.add_argument("app", type=Path)
    assets = commands.add_parser("assets")
    assets.add_argument("tag")
    assets.add_argument("directory", type=Path)
    notes = commands.add_parser("notes")
    notes.add_argument("tag")
    notes.add_argument("commit")
    args = parser.parse_args()
    try:
        if args.command == "version":
            number = validate_version(args.tag)
            print(f"Validated Unwrap {number}")
            if output := os.environ.get("GITHUB_OUTPUT"):
                with open(output, "a") as destination:
                    destination.write(f"tag={args.tag}\nversion={number}\n")
        elif args.command == "bundle":
            verify_bundle(args.app, args.tag, args.arch)
            print(f"Verified {args.app}")
        elif args.command == "assets":
            verify_assets(args.tag, args.directory)
            print("Both Mac DMGs and their checksums are complete.")
        elif args.command == "notes":
            print(release_notes(args.tag, args.commit))
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"Release validation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
