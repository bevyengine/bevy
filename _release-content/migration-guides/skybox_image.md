---
title: "`Skybox` `image` is now optional"
pull_requests: [23691]
---

The `image` field of the `Skybox` component now has the type `Option<Handle<Image>>` instead of `Handle<Image>`.
A `Skybox` component without an image will not draw anything, just like it was not present.

If you were creating a skybox with an image, wrap the image handle in `Some`:

```rust
// 0.18
Skybox {
    image: my_skybox,
    brightness: 1000.0,
    ..default()
}

// 0.19
Skybox {
    image: Some(my_skybox),
    brightness: 1000.0,
    ..default()
}
```

If you were previously creating a `Skybox` component with a placeholder image to be changed later, you can now remove the placeholder:

```rust
// 0.18
Skybox {
    image: cubemap_image_that_will_not_actually_be_seen,
    brightness: 1000.0,
    ..default()
}

// 0.19
Skybox {
    brightness: 1000.0,
    ..default()
}
```
