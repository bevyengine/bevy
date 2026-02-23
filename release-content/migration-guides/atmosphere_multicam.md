---
title: "Atmosphere now supports multiple cameras"
pull_requests: [23113]
---

Atmosphere now works correctly with multiple cameras. No action is required for most users.

`init_atmosphere_buffer` has been removed, and `AtmosphereBuffer` has been changed from a `Resource` to a `Component` attached to each camera entity.

If you were directly accessing `AtmosphereBuffer` as a resource, you'll need to query for it instead:

```rust
// Before
fn my_system(atmosphere_buffer: Option<Res<AtmosphereBuffer>>) {
    if let Some(buffer) = atmosphere_buffer {
        // use buffer
    }
}

// After
fn my_system(views: Query<&AtmosphereBuffer, With<Camera3d>>) {
    for buffer in &views {
        // use buffer
    }
}
```
