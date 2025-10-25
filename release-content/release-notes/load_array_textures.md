---
title: Load Array Textures
authors: ["@grind086"]
pull_requests: [21628]
---

`ImageLoader` now supports loading array textures using the new `array_layout` setting. Initial support expects a single image per row, and allows you to specify either a fixed number of rows, or a per-row pixel height.

```rs
use bevy::image::{ImageLoaderSettings, ImageArrayLayout};

let array_texture = asset_server.load_with_settings(
    "textures/array_texture.png",
    |settings: &mut ImageLoaderSettings| {
        settings.array_layout = Some(ImageArrayLayout::RowCount(4));
    },
);
```
