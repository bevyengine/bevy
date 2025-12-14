---
title: "`enable_prepass` and `enable_shadows` are now Material methods"
pull_requests: [20999]
---

The `MaterialPlugin` fields `prepass_enabled` and `shadows_enabled` have
been replaced by the `Material` methods `enable_prepass` and `enable_shadows`.

Analogous methods have also been added to `MaterialExtension`

```rust
/// before
MaterialPlugin::<MyMaterial> {
    prepass_enabled: false,
    shadows_enabled: false,
}

/// after
impl Material for MyMaterial {
    /// ...

    fn enable_prepass() { false }
    fn enable_shadows() { false }
}
```
