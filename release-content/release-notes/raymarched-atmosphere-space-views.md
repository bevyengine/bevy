---
title: Raymarched atmosphere and space views
authors: ["@mate-h"]
pull_requests: [20766]
---

(Insert screenshot of space shot including volumetric shadows)

Bevy's atmosphere now supports a raymarched rendering path that unlocks accurate views from above the atmosphere. This is ideal for cinematic shots, planets seen from space, and scenes that need sharp shadows through the volume of the atmosphere.

### What changed

- Added `AtmosphereMode::Raymarched`, as an alternative to the existing lookup texture method.
- Added support for views from above the atmosphere.

### When to choose which

- LookupTexture
  - Fastest, approximate lighting, inaccurate for long distance views
  - Ground level and broad outdoor scenes
  - Most cameras and typical view distances
  - Softer shadows through the atmosphere
- Raymarched
  - Slightly slower, more accurate lighting
  - Views from above the atmosphere or far from the scene
  - Cinematic shots that demand stable lighting over a large range of scales
  - Flight or space simulators
  - Sharp, per‑pixel shadows through the atmosphere

### How to use it

Switch the rendering method on the camera’s `AtmosphereSettings`:

```rust
use bevy::prelude::*;
use bevy::pbr::atmosphere::{Atmosphere, AtmosphereSettings, AtmosphereMode};

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Atmosphere::default(),
        AtmosphereSettings { 
          sky_max_samples: 16,
          rendering_method: AtmosphereMode::Raymarched, 
          ..Default::default() 
        }
    ));
}
```

You can also adjust the `sky_max_samples` for controlling what is the maximum number of steps to take when raymarching the atmosphere, which is `16` by default to set the right balance between performance and accuracy.

Keep the default method for most scenes. Use raymarching for cinematics, cameras positioned far from the scene, and shots requiring sharp volumetric shadows.

See the updated `examples/3d/atmosphere.rs` for a working reference.
