---
title: "`TextLayoutInfo`'s `section_rects` field has been replaced with `run_geometry`"
pull_requests: []
---

`TextLayoutInfo`'s `section_rects` field has been removed.
In its place is a new field `run_geometry` that contains the non-glyph layout geometry for a run of glyphs: the run's span index, bounding rectangle, underline position and thickness, and strikethrough position and thickness. A run in `bevy_text` is a contiguous sequence of glyphs on the same line that share the same text attributes like font, font size, and line height. The coordinates stored in `run_geometry` are unscaled and relative to the top left corner of the text layout.

Unlike the tuples of `section_rects`, `RunGeometry` does not include an `Entity` id. To find the corresponding text entity, call the `entities` method on the root text entity’s `ComputedTextBlock` component and use the `span_index` to index into the returned slice.
