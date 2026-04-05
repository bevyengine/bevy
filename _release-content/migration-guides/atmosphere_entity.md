---
title: "`Atmosphere` is now an entity"
pull_requests: [23651]
---

Previously, the `Atmosphere` component was added on the camera. Instead,
spawn an `Atmosphere` as an entity. The nearest atmosphere will be chosen for rendering.

`AtmosphereSettings` still belongs on the camera. It is the component that enables atmosphere rendering for that view.

The `scene_units_to_m` field has been removed from `AtmosphereSettings`. Use `Transform` on the `Atmosphere` entity for scale. It is inversely proportional to the old `scene_units_to_m` factor. For example, to treat one unit as 1 km (as with `scene_units_to_m: 1000.0`), set scale to `0.001`.

```rust
// 0.18
commands.spawn((
    Camera3d::default(),
    Atmosphere::earth(earth_medium),
    AtmosphereSettings {
        scene_units_to_m: 1000.0,
        ..default()
    },
));
```

```rust
// 0.19
let earth = Atmosphere::earth(earth_medium);
let scale = 0.001;
commands.spawn((
    earth,
    Transform::from_scale(Vec3::splat(scale)).with_translation(-Vec3::Y * earth.inner_radius * scale),
));

commands.spawn((
    Camera3d::default(),
    AtmosphereSettings::default(),
));
```

If you don't need to customize the scale, a default `Transform` component is added that positions the atmosphere such that the horizon lines up with the camera's default Y-up direction.

```rust
commands.spawn(Atmosphere::earth(earth_medium));
```

The `bottom_radius` and `top_radius` fields on the `Atmosphere` component have been renamed to `inner_radius` and `outer_radius` respectively to reflect their new meaning.

See the updated atmosphere example and documentation for details.
