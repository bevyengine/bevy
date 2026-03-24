---
title: "`FontAtlas` changes"
pull_requests: [23012]
---

The texture atlas layout for font atlases is no longer stored as a separate asset. Instead, it is stored directly in the `texture_atlas` field of `FontAtlas`.

The `TextureAtlasLayout` parameters of `FontAtlas`'s `new` and `add_glyph_to_atlas` methods have been removed.

`FontAtlas::add_glyph`'s offset parameter has been changed from an `IVec2` to a `Vec2`

`GlyphAtlasInfo`'s `texture_atlas` and `location` fields have been removed, replaced by `rect` and `offset` fields.

The `size` field has been removed from `PositionedGlyph`. The glyph’s size can now be obtained from the `Rect` stored in the `atlas_info: GlyphAtlasInfo` field.

`GlyphAtlasLocation::offset` is now a `Vec2`.
