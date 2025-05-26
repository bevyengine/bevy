---
title: Image resizing now returns Result
pull_requests: [19380]
---
`bevy_image::Image::resize` now returns a `Result<(), ResizeError>` to allow for an error to be returned when`image.data` is `None` indicating that the operation was a no-op due to the lack of data to resize.

Before we were setting the `image.texture_descriptor.size` even if `image.data` was `None`. Now we only set this size if `image.data` exists.

To migrate code from previous versions all calls of `Image::reize` will need to check the new `Result` return value of the method and handle it however makes sense for the surrounding context.
