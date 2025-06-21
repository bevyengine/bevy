---
title: Add `OverflowAxis::ScrollNoClip
authors: ["@hukasu"]
pull_requests: [19773]
---

Create a new variant for `OverflowAxis` called `ScrollNoClip`, which allows scrolling
while keeping overflowing items visible.

This variant are also accessible through `Overflow`'s new methods `scroll_no_clip`,
`scroll_x_no_clip`, and `scroll_y_no_clip`.
