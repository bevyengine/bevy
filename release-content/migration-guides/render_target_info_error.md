---
title: "RenderTarget error handling"
pull_requests: [20503]
---

`NormalizedRenderTargetExt::get_render_target_info` now returns a `Result`,
with the `Err` variant indicating which render target (image, window, etc)
failed to load its metadata.

This should mostly be treated as a hard error, since it indicates the rendering
state of the app is broken.
