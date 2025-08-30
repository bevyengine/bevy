---
title: "Image::reinterpret_size and Image::reinterpret_stacked_2d_as_array now return a Result"
pull_requests: [20797]
---

`Image::reinterpret_size` and `Image::reinterpret_stacked_2d_as_array` now return a `Result` instead of panicking.

Previously, calling this method on image assets that did not conform to certain constraints could lead to runtime panics. The new return type makes the API safer and more explicit about the constraints.

To migrate your code, you will need to handle the `Result` returned by `Image::reinterpret_size` or `Image::reinterpret_stacked_2d_as_array`.
