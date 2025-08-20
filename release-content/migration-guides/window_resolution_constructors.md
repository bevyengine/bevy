---
title: Window Resolution Constructors
pull_requests: [20582]
---

The `WindowResolution` type stores the width and height as `u32`. Previously, this type could only be constructed with `f32`, which were immediately converted to `u32`.
Now, `WindowResolution` can be constructed with `u32`s directly, and the pointless `f32` conversion has been removed.

```rust
WindowResolution::new(1920.0, 1080.0)
// becomes
WindowResolution::new(1920, 1080)

WindowResolution::new(some_uvec2.x as f32, some_uvec2.y as f32)
// becomes
WindowResolution::from(some_uvec2)

window_resolution: (1920.0, 1080.0).into()
// becomes
window_resolution: (1920, 1080).into()
```
