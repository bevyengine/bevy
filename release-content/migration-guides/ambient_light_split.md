---
title: "`AmbientLight` split into a component and a resource"
pull_requests: [21585]
---

The `AmbientLight` used to be both a component *and* a resource.
In 0.18, we've split this in two separate structs: `AmbientLight` and `GlobalAmbientLight`.
The resource `GlobalAmbientLight` is the default ambient light for the entire world and automatically added by `LightPlugin`.
Meanwhile, `AmbientLight` is a component that can be added to a `Camera` in order to override the default `GlobalAmbientLight`.
When appropriate, rename `AmbientLight` to `GlobalAmbientLight`.

```rust
// 0.17
app.insert_resource(AmbientLight {
    color: Color::WHITE,
    brightness: 2000.,
    ..default()
});

// 0.18
app.insert_resource(GlobalAmbientLight {
    color: Color::WHITE,
    brightness: 2000.,
    ..default()
});
```
