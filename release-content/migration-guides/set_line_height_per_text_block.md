---
title: Line height for text is now set per block using the `TextLayout` component.
pull_requests: [20333]
---

The `line_height` field has been removed from the `TextFont` component and moved to the `TextLayout` component.
All the lines in a text block have the same height, but text blocks have multiple spans, each with its own `TextFont`. Having `line_height` on `TextFont` wrongly implied that line height is set per span.
