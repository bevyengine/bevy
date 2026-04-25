---
title: "`ViewTarget`'s output accessors now return `Option`"
pull_requests: [23959]
---

`ViewTarget::out_texture`, `out_texture_color_attachment`, and `out_texture_view_format` now return `Option`. They are `None` when the render target has no output surface this frame, e.g. an occluded swap chain or a camera with `CameraOutputMode::Skip`. Nodes that blit to the output should short-circuit when `None`.
