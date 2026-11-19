---
title: SVG Assets
authors: ["@rossleonardy"]
pull_requests: [26602]
---

Bevy can now load `.svg` files as rasterized images with the `svg` feature enabled.

We want to support vector graphics for specific use cases, like UI icons.

In bsn! templates, you can use svg files like any other image:

```rust
bsn! {
    ImageNode {
        image: "branding/icon.svg"
    }
}
```

By default, SVGs are rasterized at their intrinsic size. If you want to rasterize them at another size, use `SvgLoaderSettings`:

```rust
let icon = asset_server
    .load_builder()
    .with_settings(|settings: &mut SvgLoaderSettings| {
        settings.size = SvgRasterSize::Height(48);
    })
    .load("branding/icon.svg");
```

`<text>`, `<image>` and other embedded elements are not supported.
