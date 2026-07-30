---
title: "Camera main texture changes"
pull_requests: [24280]
---

Camera rendering has been reworked to make the color target texture configurable. The main texture is now allocated at the camera's viewport size instead of the render target size. Previously the main texture matched the full render target and render passes used a viewport scissor to restrict drawing to the camera's area. Now the main texture is exactly the viewport size, so render passes draw to the entire main texture without setting a viewport (except when `MainPassResolutionOverride` is used, which changes the texture size and sets a matching viewport in 3D passes). The upscaling pass blits each camera's main texture to the output attachment.

If you want the old behavior of each camera rendering into a region of a large main texture, there is currently no direct equivalent. As an alternative, you can render multiple cameras to regions of an `Image` used as a `RenderTarget`.

Texture metadata (format, size, sample count) previously spread across `ExtractedView`, `ExtractedCamera`, `ViewTarget`, and `Msaa` is now consolidated into a single `ViewTargetInfo` component on each camera entity.

```rust
// Before
fn system(views: Query<(&ExtractedView, &ExtractedCamera, &Msaa)>) {
    for (view, camera, msaa) in &views {
        let format = view.target_format;
        let size = camera.physical_target_size.unwrap();
        let samples = msaa.samples();
    }
}

// After
use bevy_render::camera::ViewTargetInfo;
fn system(views: Query<&ViewTargetInfo>) {
    for target_info in &views {
        let format = target_info.color_format;
        let size = target_info.size;
        let samples = target_info.sample_count;

        // `ViewTarget::main_texture_format()` is also removed — use `ViewTargetInfo::color_format` instead.
    }
}
```

`Viewport::from_viewport_and_override` has been renamed to `from_main_pass_resolution_override` and no longer needs the camera's viewport:

```rust
// Before
let vp = Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override);

// After
let vp = Viewport::from_main_pass_resolution_override(resolution_override)
    .or_else(|| camera.viewport.clone());
```
