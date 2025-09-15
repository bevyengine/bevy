---
title: Specialized UI transform
pull_requests: [16615]
---

Bevy UI now uses specialized 2D UI transform components `UiTransform` and `UiGlobalTransform` in place of `Transform` and `GlobalTransform`.

`UiTransform` is a 2D-only equivalent of Transform with a responsive translation in `Val`s. `UiGlobalTransform` newtypes `Affine2` and is updated in `ui_layout_system`.

`Node` now requires `UiTransform` instead of `Transform`. `UiTransform` requires `UiGlobalTransform`.

The `UiTransform` equivalent of the `Transform`:

```rust
Transform {
    translation: Vec3 { x, y, z },
    rotation:Quat::from_rotation_z(radians),
    scale,
}
```

is

```rust
UiTransform {
    translation: Val2::px(x, y),
    rotation: Rot2::from_rotation(radians),
    scale: scale.xy(),
}
```

In previous versions of Bevy `ui_layout_system` would overwrite UI node's `Transform::translation` each frame. `UiTransform`s aren't overwritten and there is no longer any need for systems that cache and rewrite the transform for translated UI elements.

If you were relying on the `z` value of the `GlobalTransform`, this can be derived from `UiStack` instead.
