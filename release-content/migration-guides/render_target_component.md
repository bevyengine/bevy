---
title: "`RenderTarget` is now a component"
pull_requests: [20917]
---

`RenderTarget` has been moved from a field on `Camera` to a separate required component.

When spawning a camera, specify `RenderTarget` as a component instead of setting `camera.target`:

```rust
// before
commands.spawn((
    Camera3d::default(),
    Camera {
        target: RenderTarget::Image(image_handle.into()),
        ..default()
    },
));

// after
commands.spawn((
    Camera3d::default(),
    RenderTarget::Image(image_handle.into()),
));
```
