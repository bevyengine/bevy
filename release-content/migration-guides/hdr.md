---
title: "`Hdr` moved to `bevy_camera`"
pull_requests: [22683]
---

`Hdr` has been moved from `bevy_render` to `bevy_camera`.

Furthermore, it is no longer extracted to the render world. If you were relying on its presence in the render world, consider using `ExtractedCamera::hdr` instead.
