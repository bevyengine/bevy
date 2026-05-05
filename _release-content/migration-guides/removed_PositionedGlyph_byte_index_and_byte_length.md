---
title: "`PositionedGlyph`'s `byte_index` and `byte_length` fields have been removed"
pull_requests: [23695]
---

`PositionedGlyph`'s `byte_index` and `byte_length` fields have been removed. Unlike Cosmic Text, `Parley` doesn't expose these values in its `GlyphRun`s.

If needed, these range can be retrieved using `visual_clusters` by mapping each cluster's `text_range` to its corresponding `Glyph`(s). However, this approach is quite fragile.
