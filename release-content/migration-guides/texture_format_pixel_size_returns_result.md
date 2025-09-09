---
title: TextureFormat::pixel_size now returns a Result
pull_requests: [20574]
---

The `TextureFormat::pixel_size()` method now returns a `Result<usize, TextureAccessError>` instead of `usize`.

This change was made because not all texture formats have a well-defined pixel size (e.g. compressed formats). Previously, calling this method on such formats could lead to runtime panics. The new return type makes the API safer and more explicit about this possibility.

To migrate your code, you will need to handle the `Result` returned by `pixel_size()`.
