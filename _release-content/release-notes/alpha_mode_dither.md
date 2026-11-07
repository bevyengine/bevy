---
title: Dithered alpha mode
authors: ["@barsoosayque"]
pull_requests: [26441]
---

`AlphaMode::Dither` is a new alpha mode that approximates transparency by discarding a fraction of an object's pixels proportional to its alpha, following a 4x4 ordered dithering pattern (also known as "screen-door transparency").

Dithered objects are rendered in the opaque pass: they write depth, cast (dithered) shadows, and work with the depth prepass and deferred rendering.
Unlike `AlphaMode::AlphaToCoverage`, dithering doesn't require MSAA, and it preserves partial transparency in base color textures.

Each mesh instance uses its own, stable variation of the dithering pattern, so overlapping dithered objects can still be seen through one another.

This makes it a cheap way to fade objects in and out, for example when they occlude the camera, or when they spawn and despawn. Note that the dithering pattern is fixed in screen space, so it remains visible up close.

```rust
StandardMaterial {
    base_color: Color::srgba(1.0, 1.0, 1.0, 0.5),
    alpha_mode: AlphaMode::Dither,
    ..default()
}
```
