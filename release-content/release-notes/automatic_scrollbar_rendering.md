---
title: Automatic scrollbar rendering
authors: ["@ickshonpe"]
pull_requests: [21897]
---

Scrollbars are now drawn automatically for nodes with scrolled content and a `scrollbar_width` greater than 0.

Styling can be set using the `ScrollbarStyle` component. For now, it only supports changing the scrollbar's colors.

`ComputedNode` has new methods that can be used to compute the geometry for a UI node's scrollbars:

* `horizontal_scrollbar_gutter`
* `vertical_scrollbar_gutter`
* `horizontal_scrollbar_thumb`,
* `vertical_scrollbar_thumb`
