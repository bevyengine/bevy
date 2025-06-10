---
title: RelativeCursorPosition is object-centered
pull_requests: [16615]
---

`RelativeCursorPosition`'s coordinates are now object-centered with (0,0) at the the center of the node and the corners at (±0.5, ±0.5). Its `normalized_visible_node_rect` field has been removed and replaced with a new `cursor_over: bool` field which is set to true when the cursor is hovering a visible section of the UI node.
