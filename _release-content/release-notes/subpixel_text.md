---
title: RGB Subpixel Antialiased Text
authors: ["@masegraye"]
pull_requests: []
---

Bevy can now render text with **RGB subpixel antialiasing**, the technique
behind the visibly crisper text you see in Zed, macOS native apps, and most
modern code editors. Opt in per-`Text` (or `Text2d`) by setting
`TextFont::font_smoothing = FontSmoothing::SubpixelAntiAliased`.

Bevy's existing grayscale AA (`FontSmoothing::AntiAliased`, the default) has
an effective horizontal resolution of one physical pixel — a glyph stem that
falls between pixels gets softened into a two-pixel-wide gray blur. Subpixel
AA treats each pixel as three horizontal colour sub-pixels (R, G, B, as
physically arranged on most LCD/OLED panels) and addresses them
independently, roughly tripling the effective horizontal resolution. At 10pt
and 14pt body text the difference is clearly visible: sharper vertical stems,
better digit legibility, no "dancing" between pixel rows. The tradeoff is a
subtle rainbow halo on contrast edges, which is why it is opt-in rather than
the default.

## How to enable

```rust
use bevy::prelude::*;
use bevy::text::FontSmoothing;

commands.spawn((
    Text::new("Sharper body text"),
    TextFont {
        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
        font_size: FontSize::Px(14.0),
        font_smoothing: FontSmoothing::SubpixelAntiAliased,
        ..default()
    },
));
```

The same `FontSmoothing` variant works for `Text2d` (world-space sprite
text) without any extra setup — both the UI and sprite render pipelines
share the glyph atlas and the RGB coverage path.

## What's in the feature

- **UI text + `Text2d` support.** Both `bevy_ui_render` and
  `bevy_sprite_render` gain a dual-source-blend subpixel pipeline variant
  selected per extracted item when a glyph's section uses
  `FontSmoothing::SubpixelAntiAliased`.
- **`SubpixelTextSettings` resource** for app-level tuning. Two fields —
  `enhanced_contrast: f32` (default `0.5`) and `gamma_ratios: Vec4` (the
  gamma=1.8 row of GPUI's `GAMMA_INCORRECT_TARGET_RATIOS`). Apps targeting
  specific display/background/foreground combinations (dark IDE, light
  reading app) can tune these once and both pipelines pick up the change.
- **`SubpixelLcdLayout` resource** for panel orientation. Two variants:
  `HorizontalRgb` (default, covers ~99% of desktop LCDs) and
  `HorizontalBgr` (some older displays). Vertical-subpixel variants are
  deliberately deferred — correct vertical-subpixel AA needs a rasteriser
  rotation or a second atlas, which is better handled in a dedicated
  follow-up than bundled here.
- **Per-subpixel-bucket atlas partitioning** via the `subpixel_bucket`
  field on `GlyphCacheKey`. Glyphs rasterised at different horizontal
  sub-pixel offsets (four bins: `Zero`, `Quarter`, `Half`, `ThreeQuarter`)
  land in separate atlas cells so shaping at fractional positions stays
  crisp. Non-subpixel smoothing modes use `SubpixelBucket::NotApplicable`
  and retain the prior single-cell-per-glyph layout.
- **Graceful DSB-unavailable fallback.** On adapters where
  `wgpu::Features::DUAL_SOURCE_BLENDING` is unavailable (WebGL2, some
  older mobile Vulkan, DX11), the `SubpixelCapable` resource is `false`
  and `SubpixelAntiAliased` requests transparently downgrade to the
  grayscale pipeline so apps continue to render correctly.

## How it works

On `SubpixelAntiAliased`, `get_outlined_glyph_texture` calls
`swash::scale::Render::new(...).format(Format::Subpixel).offset(...)`
instead of the default `Format::Alpha` path, producing an RGB coverage
atlas where each of the three channels represents coverage of one subpixel
column. The extended `GlyphCacheKey` gives each of the four horizontal
subpixel bins its own atlas cell, and the render pipelines select a
dual-source-blend (DSB) shader variant that emits per-channel source and
per-pixel alpha in a single pass — the GPU then blends against the
framebuffer using `Src1 / OneMinusSrc1` colour and `One / OneMinusSrcAlpha`
alpha. The WGSL helpers (`color_brightness`,
`apply_contrast_and_gamma_correction3`, `swizzle_subpixel_atlas`, …) are
ported directly from GPUI's Metal shader.

Detect DSB support at runtime via the `bevy_text::SubpixelCapable`
resource, which is inserted by both `bevy_ui_render::init_ui_subpixel_capability`
and `bevy_sprite_render::init_sprite_subpixel_capability` at `RenderStartup`.

## Limitations

- **Vertical-subpixel panels.** Rotated portrait panels and vertical-stripe
  OLEDs are not currently supported; the atlas is pre-offset horizontally
  by swash and would need a rotated rasterisation path for vertical
  fringing to line up correctly. A future spec may revisit this.
- **LCD-orientation auto-detection.** No platform-specific probing today —
  `SubpixelLcdLayout` is a manual override. Auto-detection would require
  per-OS APIs that are fiddly enough to be their own future work.
- **Adapter feature requirement.** Subpixel AA relies on
  `wgpu::Features::DUAL_SOURCE_BLENDING`. Adapters without DSB fall back
  to grayscale AA instead of the subpixel pipeline; apps render correctly
  but without the extra horizontal resolution.

## Examples

See `examples/ui/text_subpixel.rs` for a UI-side side-by-side comparison of
the three `FontSmoothing` variants at four font sizes, with interactive
keyboard controls for `enhanced_contrast` (`1`/`2`/`3`),
`SubpixelLcdLayout` (`R`/`B`), and ad-hoc screenshot capture (`S`). See
`examples/2d/text2d_subpixel.rs` for the `Text2d` / world-space
counterpart.
