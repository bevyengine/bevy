---
title: Automatically allocate space for scrollbars
authors: ["@ickshonpe"]
pull_requests: [20093]
---

`Node` has a new field `scrollbar_width`. If `OverflowAxis::Scroll` is set for a UI Node's axis, a space for a scrollbars of width `scrollbar_width` will automatically be left in the layout.
