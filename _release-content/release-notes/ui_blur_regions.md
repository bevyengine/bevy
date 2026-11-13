---
title: UI Blur Regions
authors: ["@joelawm"]
pull_requests: [24609]
---

Bevy UI now supports blurring regions of the screen behind UI nodes, making it easy to create frosted-glass panels, HUD overlays, and translucent menus over live 2D or 3D scenes!

To enable blur regions, add [`BlurRegionCamera`](https://docs.rs/bevy/latest/bevy/ui_render/struct.BlurRegionCamera.html) to your camera with your chosen [`BlurSetting`](https://docs.rs/bevy/latest/bevy/ui_render/enum.BlurSetting.html), and tag any UI node you want blurred behind with [`BlurRegion`](https://docs.rs/bevy/latest/bevy/ui_render/struct.BlurRegion.html):

```rust
use bevy::prelude::*;
use bevy::ui_render::{BlurRegion, BlurRegionCamera, BlurSetting};

fn setup(mut commands: Commands) {
    // Enable blur region rendering on the camera (Camera2d works too!)
    commands.spawn((
        Camera3d::default(),
        BlurRegionCamera::new(BlurSetting::GAUSSIAN),
    ));

    // Any UI node with `BlurRegion` becomes a frosted-glass pane
    commands.spawn((
        BlurRegion,
        Node {
            width: px(300.0),
            height: px(200.0),
            border_radius: BorderRadius::all(px(16.0)),
            ..default()
        },
        // Optional: add a translucent background color to tint the blurred region
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.1)),
    ));
}
```

Each frame, Bevy automatically syncs the layout rectangles and border radii (including rounded and elliptical corners) of visible [`BlurRegion`](https://docs.rs/bevy/latest/bevy/ui_render/struct.BlurRegion.html) nodes and applies the blur as a post-processing pass before drawing UI on top. You can also push custom screen-space rectangles directly onto [`BlurRegionCamera`](https://docs.rs/bevy/latest/bevy/ui_render/struct.BlurRegionCamera.html) via `blur` and `rounded_blur`.

### Four Built-in Blur Algorithms

Different games and UI styles have different needs, so [`BlurSetting`](https://docs.rs/bevy/latest/bevy/ui_render/enum.BlurSetting.html) comes with four algorithms that let you trade off between performance, blur width, and visual style—all of which can be tweaked at runtime:

- **`BlurSetting::Gaussian`** *(default)*: A two-pass separable Gaussian blur that uses bilinear filtering to halve the required texture samples. Produces clean, classic frosted glass.
- **`BlurSetting::DualKawase`**: Downsamples the scene through a mip chain of half-resolution textures and upsamples back. Produces very wide, smooth blurs at a fraction of the cost of an equivalent Gaussian blur.
- **`BlurSetting::Bokeh`**: A separable complex-kernel convolution that mimics a camera aperture, blooming out-of-focus highlights into bright, hard-edged discs rather than smearing them out.
- **`BlurSetting::BoxBlur`**: A simple two-pass separable box filter where every sample in the kernel contributes equally.

Check out the new `blur` example to compare all four algorithms and tweak their parameters live:

```sh
cargo run --example blur --features bevy_feathers
```
