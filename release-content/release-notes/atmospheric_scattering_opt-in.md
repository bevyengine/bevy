---
title: Use `VolumetricLight` to opt-in for atmospheric scattering
authors: ["@hukasu"]
pull_requests: [21839]
---

For a `DirectionalLight` to influence in `Atmosphere` it now requires to have the
`VolumetricLight` component.

```rust
commands.spawn((
    DirectionalLight::default(),
    Transform::from_translation(Vec3::new(0., 5., -15.)).looking_at(Vec3::ZERO, Vec3::Y),
    VolumetricLight,
));
```
