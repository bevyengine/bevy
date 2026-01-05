---
title: "Generalized Atmospheric Scattering Media"
authors: ["@ecoskey"]
pull_requests: [20838]
---

Most of the fields on `Atmosphere` have been removed in favor of a handle
to the new `ScatteringMedium` asset.

```rust
// 0.17
pub struct Atmosphere {
    pub bottom_radius: f32,
    pub top_radius: f32,
    pub ground_albedo: Vec3,
    // All of these fields have been removed.
    pub rayleigh_density_exp_scale: f32,
    pub rayleigh_scattering: Vec3,
    pub mie_density_exp_scale: f32,
    pub mie_scattering: f32,
    pub mie_absorption: f32,
    pub mie_asymmetry: f32,
    pub ozone_layer_altitude: f32,
    pub ozone_layer_width: f32,
    pub ozone_absorption: Vec3,
}

// 0.18
pub struct Atmosphere {
    pub bottom_radius: f32,
    pub top_radius: f32,
    pub ground_albedo: Vec3,
    // This replaces all of the old fields.
    pub medium: Handle<ScatteringMedium>,
}
```

Unfortunately, this means `Atmosphere` no longer implements `Default`. Instead,
you can still access the default earthlike atmosphere through the
`EarthlikeAtmosphere` resource:

```rust
fn setup_camera(
    mut commands: Commands,
    earthlike_atmosphere: Res<EarthlikeAtmosphere>
) {
    commands.spawn((
        Camera3d,
        earthlike_atmosphere.get(),
    ));
}
```
