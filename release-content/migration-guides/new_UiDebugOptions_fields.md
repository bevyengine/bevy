---
title: "New fields added to `UiDebugOptions`"
pull_requests: [21931]
---

`UiDebugOptions` has new `bool` fields: `outline_border_box`, `outline_padding_box`, `outline_content_box`,`outline_scrollbars`, and `ignore_border_radius`. To match the previous behavour of `UiDebugOptions`, where only the border box outline was rendered, use the default values with `outline_border_box: true` and the rest of the new fields set to false.
