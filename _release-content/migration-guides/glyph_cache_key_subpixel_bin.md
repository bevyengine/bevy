---
title: "`GlyphCacheKey` has a new `subpixel_bin` field"
pull_requests: [26010]
---

`GlyphCacheKey` has a new `subpixel_bin` field used to rasterize glyphs at different horizontal subpixel offsets. Use `GlyphCacheKey::new` to get both the cache key and the snapped pixel position to draw the glyph at.

`add_glyph_to_atlas` and `get_outlined_glyph_texture` now also take the full `GlyphCacheKey` instead of using the id directly.
