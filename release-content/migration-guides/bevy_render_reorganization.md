---
title: `bevy_render` reorganization
pull_requests: [19949]
---

You must now import `ToNormalizedRenderTarget` to use `RenderTarget::normalize`
`ManualTextureViews` is now in `bevy_render::texture`
Camera and visibility types have been moved to a new crate, `bevy_camera`, but continue to be re-exported by bevy_render.
