---
title: "Changes have been made to `DepthAttachment`, `ViewDepthTexture` and `ViewPrepassTextures::depth` to accommodate stencil support"
pull_requests: [24725]
---

- The original `DepthAttachment` has been renamed to `DepthStencilViewAttachment`, which holds a new `DepthStencilViews` instead of a `TextureView`. It requires specifying views for the combined depth-stencil, depth-only, and stencil-only aspects, as well as a clear value for stencil.
- A new `DepthStencilAttachment` has been introduced, containing a `CachedTexture` and its corresponding `DepthStencilViewAttachment`, along with an optional depth texture and views from the previous frame.
- `ViewDepthTexture` has been renamed to `ViewDepthStencilTexture` which holds a `DepthStencilAttachment` instead of a `Texture`.
- `ViewPrepassTextures::depth` is now `Option<DepthStencilAttachment>` instead of `Option<ColorAttachment>`.

To ensure compatibility with future custom depth formats, such as combined depth-stencil and stencil-only formats,
choose between a single-aspect view (for resource binding in shaders) and an all-aspect view (for render attachments) based on how it will be used.
