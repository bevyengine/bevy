---
title: "BorderRadius_and_ResolvedBorderRadius_fields_are_now_2d"
pull_requests: [24779]
---

In order to support elliptical nodes, the fields of `BorderRadius` are now `Val2`s and the fields of `ResolvedBorderRadius` are now `Vec2`s.

```rust
BorderRadius {
    pub top_left: px(10.),
    pub top_right: percent(20.),
    pub bottom_right: zero(),
    pub bottom_left: vh(5.),
}
```

becomes

```rust
BorderRadius {
    pub top_left: Val2::all(px(10.)),
    pub top_right: Val2::all(percent(20.)),
    pub bottom_right: Val2::all(zero()),
    pub bottom_left: Val2::all(vh(5.)),
}
```

The `BorderRadius` constructor and update functions are no longer `const`. This is so the params can take an `Into<Val2>`:

```rust
let n = BorderRadius::top_right(vh(10.));
let m = BorderRadius::top_right(Val2::all(px(10.), px(20.)));
```
