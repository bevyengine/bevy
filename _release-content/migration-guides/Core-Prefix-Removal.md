---
title: "`Core` prefix removed from UI widget components"
pull_requests: [23612, 23938]
---

`CoreScrollbarThumb` has been renamed to `ScrollbarThumb`.
`CoreScrollbarDragState` has been renamed to `ScrollbarDragState`.
`CoreSliderDragState` has been renamed to `SliderDragState`.

Additionally, `ScrollbarThumb` nodes are now laid out after `ui_layout_system` by `update_scrollbar_thumb`. `ScrollbarThumb` entities do
not have a `Node` component. The only layout options are for borders, which can be set using `ScrollbarThumb`'s new
`border` and `border_radius` fields.
