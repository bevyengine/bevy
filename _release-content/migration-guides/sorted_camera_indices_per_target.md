---
title: "`SortedCamera::hdr` removed: camera indices count per render target"
pull_requests: [25479]
---

In a camera stack that mixes `Hdr` on one render target, the upper camera used to
overwrite the lower camera's output. Its blit now auto-detects alpha blending and
composites over the base. Set an explicit `blend_state` in `CameraOutputMode::Write` to
keep the old replace behavior. With `MsaaWriteback::Auto`, the upper camera's writeback
also runs even when it clears. Single cameras and uniform-`Hdr` stacks are unaffected.

`SortedCamera::hdr` has been removed, and `sorted_camera_index_for_target` now counts
per render target alone. A render-world system that read the field should read
`ExtractedCamera::hdr` on the view entity instead.
