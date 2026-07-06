---
title: "Elliptical Border Radius"
authors: ["@ickshonpe"]
pull_requests: [24779]
---

Bevy UI now supports Nodes with elliptical border geometry.

The fields of `BorderRadius` are now `Val2`s to enable different radii to be set for each axis.

The `BorderRadius` constructor and update functions are no longer `const`, and their parameters take `Into<Val2>`s instead of `Val`s. `Val2` now implements `From<Val>` so existing code using border radius shouldn't require any changes:

```rust
let a = BorderRadius::all(Val2::all(vh(10.)));
let b = BorderRadius::all(vh(10.)); // a == b
let c = BorderRadius::top_right(Val2::all(px(10.), px(20.)));
```
