---
title: `AtmosphericScattering` component for opting-in a `DirectionalLight` into influencing in `Atmosphere`
authors: ["@hukasu"]
pull_requests: [21839]
---

For a `DirectionalLight` to influence in `Atmosphere` it now requires to have the
`AtmosphericScattering` component.

```rust
commands.spawn((
    DirectionalLight::default(),
    Transform::from_translation(Vec3::new(0., 5., -15.)).looking_at(Vec3::ZERO, Vec3::Y),
    AtmosphericScattering,
));
```
