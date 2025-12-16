---
title: "`ExtractedUiNode`'s `stack_index` has been renamed to `z_order` and is now an `f32`."
pull_requests: [19691]
---

`ExtractedUiNode`’s `stack_index` field has been renamed to `z_order` and its type changed from `u32` to `f32`.
Previously `stack_index` would be converted into an `f32` after extraction during the `Render` schedule, then
offsets would be applied to determine draw order before sorting (lowest value rendered first).
For example, a node's fill color is given an offset of `0.` and a box shadow is given an offset of `-0.1`, so that
the shadow will be drawn behind the node.

Changing the field to an `f32`, enables finer control of the UI draw order by allowing these offsets to be applied during extraction,
and fixes a bug affecting the ordering of texture-sliced nodes.
