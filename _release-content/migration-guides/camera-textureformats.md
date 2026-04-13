---
title: "Camera TextureFormat rework"
pull_requests: [23734]
---

`ExtractedView::hdr` has been moved to `ExtractedCamera::hdr`. Views do not have a notion
of HDR, that is a camera-specific property.
`TextureFormat::bevy_default()` and `ViewTargets::TEXTURE_FORMAT_HDR` are deprecated,
please source your texture format from `ExtractedView::target_format` instead, and
plumb it through your specialization keys.
Similarly, `ViewTarget::is_hdr` was removed. Use `ExtractedCamera::hdr` to check this, as it is a property of a camera not a view target.
