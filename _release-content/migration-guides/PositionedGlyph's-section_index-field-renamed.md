---
title: "`PositionedGlyph::span_index` is now `section_index`"
pull_requests: [23381]
---

Only a `TextSpan` entity should be referred to as a "span". Use "section" when an entity could be either a text root or a `TextSpan`.  Hence, `PositionedGlyph::span_idex` has been renamed to `section_index`.
