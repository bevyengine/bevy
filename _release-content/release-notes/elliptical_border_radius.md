---
title: "Elliptical Border Radius"
authors: ["@ickshonpe"]
pull_requests: [24779]
---

Bevy UI can now draw Nodes with elliptical border geometry.

The fields of `BorderRadius` are now `CornerRadius`s to enable different radius to be set for each axis.

The `BorderRadius` constructor and update functions are no longer `const`, and their parameters take `Into<CornerRadius>`s instead of `Val`s. `CornerRadius` now implements `From<Val>` so most existing code using border radius shouldn't require changes:

```rust
let a = BorderRadius::all(CornerRadius::circular(vh(10.)));
let b = BorderRadius::all(vh(10.)); // a == b
let c = BorderRadius::top_right(CornerRadius::new(px(10.), px(20.)));
```
