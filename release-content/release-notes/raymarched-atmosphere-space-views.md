---
title: Raymarched atmosphere and space views
authors: ["@mate-h"]
pull_requests: [20766]
---

(Insert screenshot of space shot including volumetric shadows)

Bevy's atmosphere now supports a raymarched rendering path that unlocks accurate views from above the atmosphere. This means **Bevy 0.17** now has two atmosphere rendering modes to choose from:

- [`AtmosphereMode::Raymarched`]
  - Ideal for cinematic shots, planets seen from space, and "flight simulator" type scenes
  - More accurate lighting, but slower
  - Sharper shadows through the atmosphere
- [`AtmosphereMode::LookupTexture`]
  - This is the default
  - Great for ground level and broad outdoor scenes
  - Less accurate lighting at long distances, but faster
  - Softer shadows through the atmosphere

To use it, add an [`Atmosphere`] component to your [`Camera`] and set the rendering method on the camera’s [`AtmosphereSettings`]:

```rust
commands.spawn((
    Camera3d::default(),
    Atmosphere::default(),
    AtmosphereSettings { 
      rendering_method: AtmosphereMode::Raymarched, 
      ..default() 
    }
));
```

You can also adjust the `AtmosphereSettings::sky_max_samples` to configure the maximum number of steps to take when raymarching the atmosphere. Lower numbers are faster and less accurate. Higher numbers are slower and more accurate.

See the updated [`atmosphere` example](https://github.com/bevyengine/bevy/blob/release-0.17.0/examples/3d/atmosphere.rs) for a working reference.
