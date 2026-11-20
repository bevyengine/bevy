---
title: Partial texture atlas builds
authors: ["@masamori0083"]
pull_requests: [23467]
---

`TextureAtlasBuilder` now supports partial builds via `build_partial()`.

Previously, `build()` would fail with `TextureAtlasBuilderError::NotEnoughSpace` if all textures could not fit into the atlas, producing no result.

With `build_partial()`, the builder instead returns a texture atlas containing the successfully placed textures, along with a list of textures that could not be placed.

This allows users to gracefully handle atlas size limits without losing all results, making it easier to work with large or variable sets of textures.

The existing `build()` method remains unchanged and continues to return an error if not all textures fit.
