---
title: "CoreScrollbarThumb is now `ScrollbarThumb` and scroll bar thumb layout is updated separately"
pull_requests: [23612]
---

`CoreScrollbarThumb` has been renamed to `ScrollbarThumb`.

`ScrollbarThumb` nodes are now laid out after `ui_layout_system` by `update_scrollbar_thumb`. `ScrollbarThumb` entities do
not have a `Node` component. The only layout options are for borders, which can be set using `ScrollbarThumb`'s new
`border` and `border_radius` fields.
