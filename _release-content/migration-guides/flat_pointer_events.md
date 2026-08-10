---
title: "Flat Pointer Events"
pull_requests: [25337]
---

Pointer events are now "flattened". For example `Pointer<Press>` is now `PointerPress`. These events no longer use (or implement) `Deref` to access the inner `Press` fields. Instead they are stored directly on the `PointerPress` event. `Pointer` is now non-generic, and is a field stored on each pointer event.

```rust
// Before
fn on_press(press: On<Pointer<Press>>) {
  info!("pressed {} {:?} {:?}", press.entity,  press.pointer_id, press.pointer_location.position);
} 

// After
fn on_press(press: On<PointerPress>) {
  info!("pressed {} {:?} {:?}", press.entity, press.pointer.id,  press.pointer.position);
}
```

`pointer.position` is just a `Vec2`, rather than a `Location`. `Location` isn't used much in practice. Consumers that need a `Location` can now use `pointer.location()`.
