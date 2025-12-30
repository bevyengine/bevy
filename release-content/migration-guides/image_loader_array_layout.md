---
title: Image Loader Array Layout
pull_requests: [21628]
---

`ImageLoader` now supports loading array textures using the new `ImageLoaderSettings::array_layout` setting.

In previous versions, loading an array texture generally required a system that waited for the asset to load, then called `Image::reinterpret_stacked_2d_as_array`. Now the `ImageLoader` can do that for you automatically.

```rs
use bevy::image::{ImageLoaderSettings, ImageArrayLayout};

let array_texture = asset_server.load_with_settings(
    "textures/array_texture.png",
    |settings: &mut ImageLoaderSettings| {
        // Load the image as a stacked array of 4 textures.
        settings.array_layout = Some(ImageArrayLayout::RowCount { rows: 4 });
    },
);
```
