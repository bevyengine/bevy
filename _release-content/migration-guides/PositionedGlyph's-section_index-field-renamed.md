---
title: "`PositionedGlyph::span_index` is now `section_index`"
pull_requests: [23381]
---

`PositionedGlyph::span_idex` has been renamed to `section_index`, because only a `TextSpan` entity should be referred to as a "span". We use "section" when an entity could be either a text root or a `TextSpan`.
