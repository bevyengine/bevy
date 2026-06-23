---
title: "Changes to `DepthAttachment`, `ViewDepthTexture` and `ViewPrepassTextures::depth`"
pull_requests: [24725]
---

- The original `DepthAttachment` has been renamed to `DepthViewAttachment`, which now holds a `DepthStencilViews` instead of a `TextureView`. It requires specifying views for the combined depth-stencil, depth-only, and stencil-only aspects, as well as a clear value for stencil.
- A new `DepthAttachment` has been introduced, containing a `CachedTexture` and its corresponding `DepthViewAttachment`, along with an optional depth texture and views from the previous frame.
- `ViewDepthTexture` now holds the new `DepthAttachment` instead of a `Texture`.
- `ViewPrepassTextures::depth` is now `Option<DepthAttachment>` instead of `Option<ColorAttachment>`.

To ensure compatibility with future custom depth formats, such as combined depth-stencil and stencil-only formats,
choose between a single-aspect view (for resource binding in shaders) and an all-aspect view (for render attachments) based on how it will be used.
