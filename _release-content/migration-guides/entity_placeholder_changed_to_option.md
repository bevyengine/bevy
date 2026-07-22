---
title: "Some `Entity::PLACEHOLDER` have been replaced with `Option<Entity>`"
pull_requests: []
---

A number of variables and functions that used `Entity::PLACEHOLDER` as a null
value have been changed to use `None` instead:

- `UiCameraMapper::current_camera()`
- `RetainedViewEntity::auxiliary_entity`

```rust
// Bevy 0.19
let camera: Entity = camera_mapper.current_camera();
if camera != Entity::PLACEHOLDER {
    ...
}

// Bevy 0.20
if let Some(camera) = camera_mapper.current_camera() {
    ...
}
```
