---
title: "Main textures are the size of the camera's viewport"
pull_requests: []
---

A camera's main texture used to be the size of its render target, and the camera drew into the part
of it covered by its viewport. Now the main texture is the size of the viewport, the camera draws
into all of it, and the viewport only decides where on the render target the main texture is written.
Cameras with viewports at different positions no longer share an implicit main texture.

The `ExtractedCamera::physical_viewport_size` method is replaced by
`ExtractedCamera::main_texture_size`, and `ExtractedCamera::physical_target_size` is renamed to
`ExtractedCamera::target_size`. Neither is an `Option`. Size textures that match the main texture
with `main_texture_size`.

`ExtractedCamera::viewport` is renamed to `ExtractedCamera::target_viewport`, and is the camera's
viewport on the render target. Render passes that draw into the main texture should set
`ExtractedCamera::main_texture_viewport` instead, which starts at the origin of the main texture:

```rust
// Before
if let Some(viewport) = camera.viewport.as_ref() {
    render_pass.set_camera_viewport(viewport);
}

// After
if let Some(viewport) = camera.main_texture_viewport() {
    render_pass.set_camera_viewport(&viewport);
}
```

The `xy` of `ExtractedView::viewport` and `View::viewport` in shaders is now always zero. The
camera's viewport on the render target is in the new `ExtractedView::target_viewport` and
`View::target_viewport`. A shader that compares `frag_coord` with a position on the render target,
such as the cursor position in window pixels, should add `view.target_viewport.xy` to `frag_coord`.
The size of the render target is in the new `ExtractedView::target_size` and `View::target_size`.
