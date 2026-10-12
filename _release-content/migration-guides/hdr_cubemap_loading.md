---
title: HdrTextureLoaderSettings cubemap conversion
pull_requests: [25802]
---

`HdrTextureLoaderSettings` has a new `cubemap_face_size: Option<u32>` field. Add `cubemap_face_size: None` or `..Default::default()` to existing struct literals to keep loading 2D images. Existing `.hdr.meta` files do not need changes.
