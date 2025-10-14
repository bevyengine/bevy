---
title: Camera main texture format is configurable
pull_requests: [21357]
---

- `ViewTarget::TEXTURE_FORMAT_HDR`, `MeshPipelineKey::from_hdr`, `Mesh2dPipelineKey::from_hdr`, `SpritePipelineKey::from_hdr` have been removed.
- `CameraMainTextureUsages` has been removed. It is replaced by `CameraMainTextureConfig.usage`.

The main texture format of Camera can't be assumed based on whether Hdr is enabled. Instead, you should use `ExtractedView.target_format`, `*PipelineKey.view_target_format` or `ViewTarget.main_texture_format` to get the format, and create pipeline key with `*PipelineKey::from_view_target_format`.
