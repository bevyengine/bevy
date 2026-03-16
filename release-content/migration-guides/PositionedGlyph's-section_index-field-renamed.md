---
title: `PositionedGlyph::span_index` is now `section_index`
pull_requests: [23381]
---

Only `TextSpan` entities should be refered to as "spans". Entities that can be either text roots or `TextSpan`s should be called "sections". Hence, `PositionedGlyph::span_idex` has been renamed to `section_index`.
