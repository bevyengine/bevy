---
title: SVG Assets
authors: ["@rossleonardy"]
pull_requests: [26602]
---

SVGs are the standard format for vector graphics. 
There's a huge ecosystems of artists and designers that produce them,
and a lot of already-existing assets that Bevy users should be able to leverage.

Bevy can now load `.svg` files in two different ways, depending on how you want to use them:

# 1. SVGs as Image Assets
Bevy can load `.svg` files as images with the `svg` feature enabled.
The `SVGImageLoader` will load an `.svg`, rasterize it, and return a handle to an `Image` containing it.
This is the default loader for `.svg` files, and works like any other image format loader.
You can immediately use these images in UI `ImageNode`s, textures, et cetera.

In bsn! templates, you can use a path to an `.svg` file anywhere you can use an image:w

```rust
bsn! {
    ImageNode {
        image: "branding/icon.svg"
    }
}
```

SVGs are rasterized at their intrinsic size by default. If you want to rasterize them at another size, use `SvgLoaderSettings`:

```rust
let icon = asset_server
    .load_builder()
    .with_settings(|settings: &mut SvgLoaderSettings| {
        settings.size = SvgRasterSize::Height(48);
    })
    .load("branding/icon.svg");
```

By default, the SVG image loader will generate mipmaps while rasterizing an SVG. This is useful for textures and images that will be shown at another size than they're loaded at, but you can turn it off by setting `generate_mipmaps` to `false`.

```rust
let icon = asset_server
    .load_builder()
    .with_settings(|settings: &mut SvgLoaderSettigns| {
        settings.generate_mipmaps = false
    })
    .load("branding/icon.svg");
})
```

#2. SVGs as SVG document assets
Bevy can load `.svg` files as SVG document assets with the `svg` feature enabled. This way, you can keep the SVG tree in memory and have more control.

For example, you can load a single icon from a sheet of icons by using its ID:
```rust
let icon_sheet: Svg = asset_server.load("icon_sheet.svg");
let filter_icon = icon_sheet.rasterize_by_id("filter", SvgRasterSize::Intrinsic);
```

# Caveats
`<text>`, `<image>` and other embedded elements are not supported.
