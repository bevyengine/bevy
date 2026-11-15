---
title: Glyph atlas keys and helpers carry the synthetic italic skew
pull_requests: [26563]
---

If you only use `Text`, `TextFont` and the rest of the normal text API, nothing changes for you. Italic text in fonts without an italic face now renders slanted instead of upright.

Code that drives the glyph atlas directly needs two small updates:

- `FontAtlasKey` has a new field, `synthetic_skew_bits`: the synthetic italic skew in degrees, stored with `f32::to_bits`. Use `0.0_f32.to_bits()` for upright glyphs.
- `add_glyph_to_atlas` and `get_outlined_glyph_texture` take a new `synthetic_skew: Option<f32>` argument, placed before `glyph_id` in `add_glyph_to_atlas` and last in `get_outlined_glyph_texture`. Pass `None` to keep the old upright rendering.
