---
title: PointerButton is now MouseButton.
pull_requests: [25392]
---

Bevy picking used `PointerButton` for reasons...?
This was limiting as it only exposed 3 buttons.
It is also seemingly redundant, and now has been replaced with `MouseButton`.

```rust
PointerButton::Primary -> MouseButton::Left
PointerButton::Middle -> MouseButton::Middle
PointerButton::Secondary -> MouseButton::Right
```

Before:
```rust
MousePanSettings {
    enabled: true,
    button: MouseButton::Left,
}
```
After:
```rust
MousePanSettings {
    enabled: true,
    button: PointerButton::Primary,
}
```