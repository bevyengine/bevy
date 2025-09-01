---
title: Extract UI text colors per glyph
pull_requests: [20245]
---

The UI renderer now extracts text colors per glyph and transforms per text section.
`color: LinearRgba` and `translation: Vec2` have been added to `ExtractedGlyph`.
The `transform` field has moved from `ExtractedGlyph` and `ExtractedUiNode` to `ExtractedUiItem`.
The `rect` field has moved from `ExtractedUiNode` to `ExtractedUiItem`.
