---
title: "`Image` has a new `source_color_primaries` field"
pull_requests: [25472]
---

`Image` has a new `source_color_primaries: SourceColorPrimaries` field recording the color
primaries the image data is expressed in. It is metadata only and defaults to `Bt709`.
Exhaustive `Image { .. }` literals must add it, for example as
`source_color_primaries: Default::default()`.

`ImageLoaderSettings`, `ExrTextureLoaderSettings`, and `HdrTextureLoaderSettings` gained a
`source_color_primaries: Option<SourceColorPrimaries>` field, so exhaustive literals of
these structs must add it too. Existing `.meta` files need no changes.

`Image::from_buffer` and `ktx2_buffer_to_image` require a new trailing
`source_color_primaries: Option<SourceColorPrimaries>` argument. Pass `None` to use any
embedded color metadata (with `Bt709` as the fallback), or pass specific primaries to
override the metadata.
