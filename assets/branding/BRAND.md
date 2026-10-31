# Unwrap — V7 Glow

Unwrap is a lightweight terminal. Its mark is an open, geometric U: two upward-facing surfaces
join in one continuous bowl. The angled tips suggest unfolding; the diagonal join across the base
suggests layers meeting. The center remains open in every variant, including monochrome.

## Masters and geometry

- **Editable geometry:** `unwrap-symbol.svg`, 512 × 512 viewBox. The three named paths are the
  silhouette, pearl surface, and restrained inner fold. Edit these paths to change the entire family.
- **Master application PNG:** `unwrap-app-icon-1024.png`, 1024 × 1024 RGBA, identical to the primary
  `unwrap-app-icon-glow.png`. The symbol is independent of the tile and contains no text.
- **Application vector compositions:** `source/unwrap-app-icon-*.svg`, generated from the master.
- **Native container:** `unwrap.icns`, ten representations covering 16–1024 physical pixels.
- **Windows container:** `unwrap.ico`, seven representations at 16, 24, 32, 48, 64, 128 and 256 px.

The glow is a low-opacity radial gradient underneath the mark. There are no SVG filters, embedded
rasters, linked resources, or runtime font dependencies in the exported SVGs. If a renderer omits
the decorative glow gradient, the complete symbol remains intact. `unwrap-symbol-flat.svg` and
`unwrap-symbol-mono.svg` are explicit fallbacks for renderers without gradients.

## Color

| Token | Value | Use |
| --- | --- | --- |
| Background | `#0B0D0E` | Main dark background |
| Primary emerald | `#22D982` | Accent and right arm |
| Deep emerald | `#0EA362` | Lower emerald shading |
| Mint glow | `#A7F3D0` | Pearl shading and soft light |
| Light surface | `#F5F5F5` | Light backgrounds and light wordmarks |
| Secondary gray | `#8A8F98` | Secondary labels on dark backgrounds |
| Dark surface | `#15191A` | Dark supporting surfaces |

Supporting shades: `#55EBA1` at the emerald highlight, `#E0F5EB` within the pearl arm,
`#1C2321` at the tile top. Light-background artwork uses darker green pearl-side values
`#1B7451`, `#226B50`, `#0E5739` and deeper emerald values `#0D9F5F`, `#098350`, `#075F3C`
to retain the whole silhouette. Primary emerald is a brand accent,
not a normal text color on white. Use dark text on light backgrounds.

Machine-readable tokens are in `tokens.json` and `tokens.css`. These are brand artwork tokens;
they do not change the application's terminal theme or syntax colors.

## Typography

Use SF Pro through the system font APIs for native macOS interfaces. Do not redistribute SF Pro.
Use Inter for portable artwork and documentation. The wordmark is lowercase `unwrap` in Inter
Regular, with −3 px tracking at a 92 px drawing size. It is optically positioned next to a larger
symbol. The exported wordmark and lockups contain editable paths, not rasterized or live text.

The reproducible font source is Inter Regular 3.019, unmodified, under the SIL Open Font License
1.1. `source/fonts/Inter-Regular.ttf` was available locally in Ente Auth's distributed font assets;
its embedded metadata identifies The Inter Project Authors. `source/fonts/OFL.txt` accompanies it.
The font is used only for branding generation and the local preview, not added to the app's UI font
bundle. The official download could not run because the automatic approval service was unavailable.

## Clear space and minimum sizes

The symbol silhouette spans x = 96–416 and y = 88–432 in its 512-unit viewBox. Keep at least 48
units, or roughly half one arm's width, clear around the visible mark. The SVG has this space built
in. Keep at least one lowercase `u` width free around lockups. Never crop into the U opening.

- Standalone symbol: 16 px minimum, using `favicon-16.png` or `favicon.svg` at the smallest sizes.
- Full shaded symbol: 48 px minimum. Glow and fold shading are optional detail, not identification.
- Horizontal lockup: 160 px minimum width; stacked: 96 px minimum width.
- Outline variant: 48 px minimum. Use the filled mono silhouette below this size.

The 16/32 px favicon exports use a slightly enlarged, solid deep-emerald (`#0D9F5F`) version of the same silhouette.
They omit the tile, fold shading, and glow. macOS iconset representations keep the rounded tile,
but use flat color at 16/32 physical pixels. This preserves the platform silhouette.

## Light and dark usage

`dark` in a wordmark or lockup filename means **for a dark background**, with light lettering.
`light` means **for a light background**, with dark lettering and a darker left arm. These suffixes
describe the intended background, not the color of the lettering.

Use V7 Glow as the primary dark application icon. `dark` removes the ambient glow; `light` uses a
light tile. Use black and white lockups for single-ink reproduction. `unwrap-symbol-mono.svg` uses
`currentColor` for inline embedding; external SVG images do not inherit their host element's color,
so use the explicit black or white export with `<img>`.

Do not add a wordmark inside the app icon, outline the colored symbol, stretch it, rotate it,
separate its arms, change the center opening, add neon blooms, or place pearl artwork on a pale
background. Never redraw or substitute the symbol in a preview or mockup.

## Exports

[INVENTORY.md](INVENTORY.md) lists every delivered file. [manifest.json](manifest.json) lists the
generated artwork with byte lengths and SHA-256 hashes. The kit includes:

- Seven 1024 px application PNGs: master, dark, light, glow, flat, mono and outline.
- Symbol SVGs: master, glow, flat, mono, black, white, light, outline and small; two standalone PNGs.
- Two outlined wordmark SVGs; horizontal and stacked lockups in dark, light, black and white,
  each as SVG and transparent PNG, plus the primary `unwrap-logo-lockup.svg` alias.
- PNG icons at 16, 32, 48, 64, 128, 256 and 512 px; favicon SVG/16/32 PNG; 180 px Apple touch icon.
- ICNS, ten-file intermediate iconset, ICO, six application SVG compositions, Inter and its license.
- Color tokens, generator, validator, preview, inventory, hash manifest and integration mapping.

## Repository integration

This repository is a native Rust/WarpUI app, packaged with `cargo-bundle`; it is not Electron or
Vite. The default public binary is `warp-oss`. The assets have been integrated into the OSS channel.

1. `app/Cargo.toml`, `[package.metadata.bundle.bin.warp-oss]`, now names the bundle `Unwrap` and
   references `channels/oss/icon/unwrap.icns`, the existing 512 px PNG path and the existing ICO path.
   `cargo-bundle` directly copies an explicit ICNS file instead of regenerating it from one PNG.
2. `app/src/bin/oss.rs` uses Unwrap for the embedded plist's display name and bundle name.
3. `script/macos/run` and the OSS branch of `script/macos/bundle` use `Unwrap.app`.
4. `app/channels/oss/dev.warp.WarpOss.desktop` displays Unwrap. Existing PNG/ICO paths now contain
   the new artwork; all seven Linux icon sizes are populated.
5. The five existing `app/assets/bundled/svg/warp-logo-*.svg` paths now contain Unwrap symbols and
   wordmarks. Existing auth, About and get-started views therefore load the new artwork. Their
   historical filenames remain stable. These shared assets also affect other channels.

`integration.json` records the original 14 source-to-application mappings. The broader application
rebrand is recorded in [REBRAND-STATUS.md](REBRAND-STATUS.md) and `app-integration.json`: it replaces
shared compact symbols, loading frames, all historical Dock artwork, channel artwork, installer
images, and Library onboarding illustrations. Visible local feature names are Library, Shell
Integration, Agent and Unwrap CLI. The loading component uses a pulsing SVG U; the TUI U stays
upright. macOS Reduce Motion makes the GUI loading mark and text static.

The Dock picker offers Default, Dark, Light and Mono, with aliases for older preferences. Genuine
Warp/Oz remote services, upstream attribution and third-party brands retain their names. Binary
names, bundle ID `dev.warp.WarpOss`, URL scheme `warposs`, filesystem paths and stored settings keep
their existing identifiers for compatibility. Non-OSS channel package names remain unchanged.

The OSS channel has no `AppIcon.icon` bundle. Its existing `script/compile_icon` intentionally skips
adaptive icon compilation and leaves the ICNS reference from cargo-bundle intact. The ICNS already
contains exactly one rounded-square tile. Do not put it inside a second rounded-square icon layer.
Non-OSS Icon Composer sources also contain the U, but native export validation failed on this
host for both original and replacement sources. Light, dark and monochrome alternatives are
supplied as separate assets and Dock choices. The Apple touch icon is opaque
and unrounded because iOS supplies its mask.

To build the default OSS bundle on this checkout, use:

```sh
WARP_SKIP_COMMON_SKILLS_INSTALL=1 ./script/run
```

This script selects OSS when the internal `warp-channel-config` tool is absent. With that tool
installed, it selects WarpLocal instead. To explicitly bundle OSS, run from the repository root:

```sh
cargo bundle --package warp --bin warp-oss --features gui
```

The macOS bundle is `Unwrap.app` under Cargo's active target directory, normally
`target/debug/bundle/osx/Unwrap.app`. Restart the newly built app. Historical custom Dock preferences
map to Unwrap artwork. Choose **Default** in Appearance to use the system-managed bundled icon.
macOS may retain an old Dock tile until the old application is removed from the Dock and the new
bundle opened.

## Regeneration

No generation service or large install is needed. Node 20+ with `sharp` and `@napi-rs/canvas` renders
the vectors and Inter outlines. The Python validator uses Python 3.11+ and Pillow. The ICNS and ICO
writers use documented native container headers around the exported PNG representations; native
`iconutil` rejected the valid intermediate iconset on this host, so it is not a required dependency.
The resulting ICNS was independently decoded by Pillow and macOS `sips`.

Exact commands used on this machine, from the repository root:

```sh
NODE_PATH=/Users/vaibhav/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules \
  node assets/branding/scripts/generate.cjs --integrate

NODE_PATH=/Users/vaibhav/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules \
  node assets/branding/scripts/integrate-app.cjs

/Users/vaibhav/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  assets/branding/scripts/validate.py

/Users/vaibhav/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  assets/branding/scripts/validate-app.py

/Users/vaibhav/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  assets/branding/scripts/review.py
```

On another machine with those dependencies available, use the four regeneration commands in
[REBRAND-STATUS.md](REBRAND-STATUS.md). Omit `--integrate` to render just the
kit without copying artwork into app paths. The generator does not rewrite application configuration.
Edit `unwrap-symbol.svg` for geometry/primary shading; edit `scripts/generate.cjs` for variant styling,
placement and wordmark tracking. Derived files are overwritten during regeneration.

Open `preview.html` directly in a browser, or serve just this folder locally:

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory assets/branding
```

## Verification and limits

`validation.json` records dimensions, RGBA/transparency checks, SVG parsing and path-only content,
ICNS/ICO representations, small-icon openings, file hashes, actual preview links, embedded plist
names, and agreement between generated assets and their application copies. The source icon and
lockups are visually inspected on dark/light backgrounds, including 16/32 px raster crops.

The local browser's URL policy blocked opening `preview.html`, and the shell sandbox prevented
starting a preview server. HTML references were checked structurally; browser layout was not
visually verified. `review.jpg` is a contact sheet composited from the actual exported PNGs for
offline visual inspection, not a substitute for the editable assets.

Asset validation and configuration checks do not establish that the whole application builds or
that a running Dock has refreshed. The application rebuild is blocked by the concurrent authentication/cloud cleanup; live UI
verification remains pending. [REBRAND-STATUS.md](REBRAND-STATUS.md) records the successful asset,
onboarding and isolated component checks and the outstanding validation limits.
