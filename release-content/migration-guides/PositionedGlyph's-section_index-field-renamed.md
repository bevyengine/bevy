---
title: `PostionedGlyph`'s `span_index` field is renamed to `section_index`
pull_requests: [23381]
---

Only `TextSpan` entities should be refered to as "spans". Entities that can be either text root entities or TextSpan entities should be called "sections". Hence, `PostionedGlyph::span_idex` has been renamed to `section_index`.
