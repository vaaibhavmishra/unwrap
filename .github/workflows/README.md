# Unwrap CI and releases

Unwrap currently publishes unsigned macOS desktop builds from this public repository.
The workflows use standard GitHub-hosted runners. No Apple Developer account, signing
certificate, personal access token, cloud bucket, or third-party runner account is needed.

## One-time GitHub configuration

1. Push the Unwrap source and these workflows to `main` in
   [vaaibhavmishra/unwrap](https://github.com/vaaibhavmishra/unwrap). Include the existing
   Unwrap source, icons, and resource changes, not just the workflow files. Upload the
   model files through Git LFS as part of the push (`git lfs install` enables the
   upload hook). If checkout reports missing LFS objects, run `git lfs push origin main`.
2. Confirm the repository is **public**, with `main` as its default branch.
3. In **Settings → Actions → General**, enable Actions. If the repository was forked,
   also enable workflows from the **Actions** tab. Allow the pinned actions used here:
   `actions/*`, `Swatinem/rust-cache`, `maxim-lobanov/setup-xcode`, and `taiki-e/install-action`.
4. Leave the default workflow token permission at **read repository contents**. The
   release workflow requests `contents: write` only for its draft-release job. No
   repository secret is required. If an organization policy blocks that permission,
   its owner must allow this workflow to create releases.
5. After the first CI run creates the check, add a branch ruleset for `main` requiring
   **Unwrap CI passed**. For a solo project, do not require another person's approval
   unless you have a reviewer. Allow the normal PR workflow to merge passing changes.
6. Keep release tags immutable in practice: never move or reuse a published `v*` tag.
   Optionally add a tag ruleset preventing tag updates and deletions, while allowing
   you to create new tags.
7. Enable Dependabot alerts and security updates under the repository's security
   settings. The Dependabot config needs no private registry token.

Standard runner compute is free for public repositories under GitHub's usage policy.
This repo no longer requests larger or Namespace runners. Build caches only retain
Cargo downloads; compiled targets are deliberately excluded to avoid large caches.
Temporary Actions artifacts expire after seven days. Published downloads live in
GitHub Releases, independently of temporary workflow artifacts. Git LFS has its own
storage/bandwidth allowances; check Billing if publishing large model files.

## Everyday development

1. Create a feature branch, commit changes, push it, and open a PR into `main`.
2. CI validates workflow syntax and release scripts, then runs Mac unit tests, Clippy,
   and Rust formatting checks. The app, shared core, and terminal model are covered;
   the full GUI integration suite and other operating systems are not part of this
   initial pipeline.
3. Merge once CI is green. A normal push to `main` runs CI but does not create a release.

The `ci.yml` workflow also supports **Run workflow** for manually checking `main`.

## First release and subsequent versions

The source of the release number is `[package].version` in `app/Cargo.toml`.
Use plain semantic versions such as `0.1.0` and tags such as `v0.1.0`.
Prerelease suffixes are not supported by this initial workflow; use draft releases
for installation testing before publication.

1. Set the package version in `app/Cargo.toml`. The first version is already `0.1.0`.
   For a version change, update `Cargo.lock` with Cargo and include both in the PR.
2. Merge that PR and wait for CI to pass.
3. From the up-to-date `main` branch, create and push the matching tag:

   ```bash
   git switch main
   git pull --ff-only origin main
   git tag -a v0.1.0 -m "Unwrap 0.1.0"
   git push origin v0.1.0
   ```

4. Open **Actions → Release Unwrap (macOS)**. The workflow validates the tag, checks
   the tagged commit, and builds both native architectures on standard runners:

   | Architecture | Runner | Download |
   | --- | --- | --- |
   | Apple Silicon | `macos-26` | `Unwrap-v0.1.0-macos-arm64.dmg` |
   | Intel | `macos-26-intel` | `Unwrap-v0.1.0-macos-x86_64.dmg` |

5. The workflow creates a **draft** GitHub Release only after all checks and both
   builds pass. It attaches the two DMGs and a `.sha256` file for each. The notes
   include the source commit, unsigned-installation instructions, and generated changes.
6. Download the actual release assets and complete the installation checks below.
   Edit the draft's release notes, then choose **Publish release** in GitHub.

For a failed run, use **Re-run failed jobs**. For a complete rebuild, use **Run workflow**
with the existing tag. A rerun may replace assets on an existing draft, but refuses
an already published release. If code must change, fix it on `main` and use a new
version/tag; do not retarget the old tag.

All release source checkouts use the resolved tag commit. The same tag is compiled
into the About version, written into the macOS bundle version fields, and included
in bundled version metadata. No Warp version service participates.

## Installation checks before publishing

- Verify each downloaded DMG with its checksum:

  ```bash
  shasum -a 256 -c Unwrap-v0.1.0-macos-arm64.dmg.sha256
  ```

- On a Mac of the matching architecture, mount the DMG and copy Unwrap.app to
  Applications. Launch it from Applications, not from the build directory.
- Because this build has no Apple Developer ID signature or notarization, macOS may
  require **System Settings → Privacy & Security → Open Anyway** after the first
  launch attempt. Managed machines may prohibit this. Do not disable Gatekeeper globally.
- Check the About version, create terminal tabs, run shell commands, try AI with your
  configured provider, change a setting, quit, and relaunch.
- For updates, quit the previous version before replacing it and confirm settings
  and local data remain available. Keep a previous DMG for rollback.
- Validate Intel on an Intel Mac if available; compilation and architecture checks
  alone do not establish full Intel runtime compatibility.

The workflow verifies the bundle's name, identifier, architecture, version, settings
schema, license file, and dock tile plugin. It mounts the produced DMG and compares
its app with the built app. This does not replace interactive installation testing.

## Local release build

Install Xcode with the Metal toolchain, Rust from `rust-toolchain.toml`, Git LFS,
Python 3.11 or newer, protobuf, CMake, pkgconf, and create-dmg. Install the packaging
tools with:

```bash
./script/install_cargo_bundle
./script/install_cargo_release_deps --no-build-deps
```

From the repository root, build for the native architecture:

```bash
bash script/release-macos v0.1.0 aarch64
# On an Intel Mac:
bash script/release-macos v0.1.0 x86_64
```

The version must match `app/Cargo.toml`. Output files go into `dist/`. Release builds
use optimization with LTO and debug symbols disabled by default to reduce memory
and disk pressure on standard runners. `CARGO_BUILD_JOBS` defaults to two. If a runner
runs out of memory, reduce it to one and rerun. Keep only current release assets in
`dist/` when checking a combined release with `unwrap_release.py assets`.

A clean build of this large Rust project needs substantial free disk space. The
first hosted build must establish whether the standard runner's capacity is enough;
no hosted build is implied by the presence of these workflows.

## Updates and later platforms

Installation is manual for now. The legacy Warp updater has been removed from the
current Unwrap source. Users get new versions from
[Unwrap Releases](https://github.com/vaaibhavmishra/unwrap/releases).
A future in-app notification should query this repository's releases and link to
the relevant download; automatic replacement needs its own verification design.

Windows, Linux, Web, standalone CLI, and TUI releases are not enabled in this initial
workflow. Their underlying packaging scripts remain available for later adaptation.
