---
title: "`Image` has a new `source_primaries` field"
pull_requests: [25472]
---

`Image` has a new `source_primaries: SourceColorPrimaries` field recording the color
primaries the image data is expressed in. It is metadata only and defaults to `Bt709`.
Exhaustive `Image { .. }` literals must add it, for example as
`source_primaries: Default::default()`. The image loader settings structs gain the same
field as an `Option` that defaults to `None`, so existing `.meta` files keep working.

`Image::from_buffer` and `ktx2_buffer_to_image` gain a trailing
`source_primaries: Option<SourceColorPrimaries>` parameter. It overrides the stamped
metadata, and with `None` the file's own metadata wins.
