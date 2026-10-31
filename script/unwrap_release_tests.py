import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import unwrap_release as release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def make_assets(self):
        directory = self.root / "dist"
        directory.mkdir()
        for arch in release.ARCHITECTURES:
            name = release.asset_name("v0.1.0", arch)
            asset = directory / name
            asset.write_bytes(b"test DMG contents")
            (directory / f"{name}.sha256").write_text(f"{release.sha256(asset)}  {name}\n")
        return directory

    def make_bundle(self):
        app = self.root / "Unwrap.app"
        contents = app / "Contents"
        executable = contents / "MacOS/warp-oss"
        executable.parent.mkdir(parents=True)
        executable.write_bytes(b"fixture executable")
        executable.chmod(0o755)
        with (contents / "Info.plist").open("wb") as destination:
            plistlib.dump(
                {
                    "CFBundleName": "Unwrap",
                    "CFBundleIdentifier": "dev.warp.WarpOss",
                    "CFBundleExecutable": "warp-oss",
                    "CFBundleShortVersionString": "0.1.0",
                    "CFBundleVersion": "0.1.0",
                    "WarpVersion": "v0.1.0",
                },
                destination,
            )
        resources = contents / "Resources"
        metadata = resources / "bundled/metadata"
        metadata.mkdir(parents=True)
        (metadata / "version.json").write_text(json.dumps({"warp_version": "v0.1.0"}))
        (resources / "THIRD_PARTY_LICENSES.txt").write_text("Test attribution")
        (resources / "settings_schema.json").write_text('{"type":"object"}')
        (contents / "PlugIns/WarpDockTilePlugin.docktileplugin").mkdir(parents=True)
        return app

    def test_versions_and_manifest_mismatch(self):
        manifest = self.root / "Cargo.toml"
        manifest.write_text('[package]\nversion = "0.1.0"\n')
        self.assertEqual(release.validate_version("v0.1.0", manifest), "0.1.0")
        with self.assertRaisesRegex(ValueError, "does not match"):
            release.validate_version("v0.2.0", manifest)

    def test_rejects_ambiguous_or_unsafe_tags(self):
        tags = (
            "0.1.0", "v01.0.0", "v0.1.0-rc.1", "v0.1.0\n", "../v0.1.0", "--help", "v$(id)"
        )
        for tag in tags:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                release.version_from_tag(tag)

    def test_requires_both_architectures(self):
        directory = self.make_assets()
        release.verify_assets("v0.1.0", directory)
        (directory / release.asset_name("v0.1.0", "x86_64")).unlink()
        with self.assertRaisesRegex(ValueError, "Missing or empty"):
            release.verify_assets("v0.1.0", directory)

    def test_rejects_corrupt_download(self):
        directory = self.make_assets()
        (directory / release.asset_name("v0.1.0", "aarch64")).write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
            release.verify_assets("v0.1.0", directory)

    def test_rejects_unexpected_assets(self):
        directory = self.make_assets()
        (directory / "other-version.dmg").write_bytes(b"stale")
        with self.assertRaisesRegex(ValueError, "Unexpected release assets"):
            release.verify_assets("v0.1.0", directory)

    def test_rejects_wrong_architecture(self):
        app = self.make_bundle()
        with patch.object(release.subprocess, "check_output", return_value="x86_64\n"):
            with self.assertRaisesRegex(ValueError, "Expected a arm64"):
                release.verify_bundle(app, "v0.1.0", "aarch64")

    def test_bundle_requires_versioned_resources(self):
        app = self.make_bundle()
        with patch.object(release.subprocess, "check_output", return_value="arm64\n"):
            release.verify_bundle(app, "v0.1.0", "aarch64")
            metadata = app / "Contents/Resources/bundled/metadata/version.json"
            metadata.write_text('{"warp_version":"v0.0.1"}')
            with self.assertRaisesRegex(ValueError, "version metadata"):
                release.verify_bundle(app, "v0.1.0", "aarch64")

    def test_rejects_missing_licenses(self):
        app = self.make_bundle()
        (app / "Contents/Resources/THIRD_PARTY_LICENSES.txt").unlink()
        with patch.object(release.subprocess, "check_output", return_value="arm64\n"):
            with self.assertRaisesRegex(ValueError, "Missing bundled resource"):
                release.verify_bundle(app, "v0.1.0", "aarch64")

    def test_rejects_stale_bundle_version(self):
        app = self.make_bundle()
        with self.assertRaisesRegex(ValueError, "CFBundleShortVersionString"):
            release.verify_bundle(app, "v0.2.0", "aarch64")

    def test_notes_identify_source_and_manual_installation(self):
        notes = release.release_notes("v0.1.0", "a" * 40)
        self.assertIn("a" * 40, notes)
        self.assertIn("unsigned and not notarized", notes)
        self.assertIn("Automatic updates", notes)

    @unittest.skipUnless(sys.platform == "darwin", "Uses macOS plutil")
    def test_plist_version_stamping(self):
        for name, tag, expected in (
            ("Unwrap", "v0.2.3", "0.2.3"),
            ("WarpDev", "v0.2025.01.07.08.02.dev_02", "0.2025.01.07.08.02.02"),
        ):
            with self.subTest(app=name):
                plist = self.root / f"{name}.plist"
                with plist.open("wb") as destination:
                    plistlib.dump(
                        {"CFBundleVersion": "0.1.0", "CFBundleShortVersionString": "0.1.0"},
                        destination,
                    )
                subprocess.run(
                    ["bash", str(release.ROOT / "script/update_plist")],
                    cwd=release.ROOT,
                    env={
                        **os.environ,
                        "GIT_RELEASE_TAG": tag,
                        "WARP_APP_NAME": name,
                        "WARP_PLIST_PATH": str(plist),
                        "WARP_PLIST_NO_FILE_TYPES": "1",
                        "WARP_SCHEME_NAME": "",
                    },
                    check=True,
                    capture_output=True,
                    text=True,
                )
                with plist.open("rb") as source:
                    result = plistlib.load(source)
                self.assertEqual(result["CFBundleShortVersionString"], expected)
                self.assertEqual(result["CFBundleVersion"], expected)
                self.assertEqual(result["WarpVersion"], tag)


if __name__ == "__main__":
    unittest.main()
