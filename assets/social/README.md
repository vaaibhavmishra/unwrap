# Unwrap social kit

Seven ready-to-post PNG graphics, vector SVG layouts, and a standalone rebranded UI preview.
The visual system uses Unwrap's existing V7 Glow mark, Inter typography, dark backgrounds,
emerald accents and pearl/mint highlights.

| Export | Dimensions | Intended use |
| --- | --- | --- |
| `unwrap-launch-landscape.png` | 1200 × 630 | X / LinkedIn post or link card |
| `unwrap-product-screenshot.png` | 1920 × 1080 | Product showcase / README / announcement |
| `unwrap-square.png` | 1080 × 1080 | Square social post |
| `unwrap-portrait.png` | 1080 × 1350 | Portrait feed post |
| `unwrap-story.png` | 1080 × 1920 | Story; copy within 250 px top/bottom safe zones |
| `unwrap-banner.png` | 1500 × 500 | Profile banner; lower-left area reserved for avatar |
| `unwrap-avatar.png` | 1024 × 1024 | Symbol-only profile image; circular-crop safe |
| `unwrap-ui-preview.png` | 1440 × 918 | Standalone rebranded screenshot composite |

All exported PNGs use sRGB. SVG files in `source/` contain vector lettering and logos, with
embedded screenshot PNGs. Lettering is outlined for portable rendering; edit the copy in
`scripts/generate.cjs` and regenerate to change wording. Screenshot contents remain raster.

## Screenshot provenance

The user requested use of the developer screenshots already present in this repository.
The screenshot source is:

`app/assets/async/png/onboarding/agent_intention/theme/theme_dark_horizontal.png`

The original is copied without changes to `source/developer-original.png`. The full preview is
cropped at x=114, y=0, width=1440, height=918, removing the old marketing reflection/background.
Its top 44-pixel tab strip is replaced with an Unwrap-branded presentation strip. The transcript,
example commands and code-editing panel remain original pixels. The square post uses a closer
crop of the code-editing panel, with a small Unwrap label above it. Upstream example command names
such as `warp-instance` remain unchanged.

These are **rebranded repository screenshot composites**, not live captures or verification of
the current local Unwrap build. They should be described as product previews when that distinction
matters. No AI model was used to invent or redraw UI. Brand source artwork was reused unchanged.

Positioning is based on the current repository README: open-source native terminal, built-in AI
agents and user-supplied API keys. No speed comparisons, benchmark scores, testimonials or privacy
guarantees have been added.

## Included copy

See `captions.md` for short and long announcement drafts and a profile bio. Add the actual public
project URL when posting; this kit does not invent one. Nothing has been posted to social accounts.

## Regeneration

From the repository root, with `sharp` and `@napi-rs/canvas` installed:

```sh
node assets/social/scripts/generate.cjs
```

The generator reads the original developer screenshot and `assets/branding/`, including its licensed
Inter font. It only writes this social kit. `manifest.json` records dimensions, file hashes and source
provenance. `review/validation.json` records export checks; every final layout was visually reviewed.
