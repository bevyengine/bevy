---
title: "BorderRadius_and_ResolvedBorderRadius_fields_are_now_2d"
pull_requests: [24779]
---

In order to support elliptical nodes, the fields of `BorderRadius` are now `CornerRadius`s and the fields of `ResolvedBorderRadius` are now `Vec2`s.

Before:
```rust
BorderRadius {
    pub top_left: px(10.),
    pub top_right: percent(20.),
    pub bottom_right: zero(),
    pub bottom_left: vh(5.),
}
```

After:
```rust
BorderRadius {
    pub top_left: CornerRadius::circular(px(10.)),
    pub top_right: CornerRadius::circular(percent(20.)),
    pub bottom_right: CornerRadius::circular(zero()),
    pub bottom_left: CornerRadius::circular(vh(5.)),
}
```

`CornerRadius` implements `From<Val>`, so you can also use `into`:

```rust
BorderRadius {
    pub top_left: px(10.).into(),
    pub top_right: percent(20.).into(),
    pub bottom_right: zero().into(),
    pub bottom_left: vh(5.).into(),
}
```

Circular corner radius is represented by setting either `CornerRadius::x` or `CornerRadius::y` to `Val::Auto`.


The `BorderRadius` constructor and update functions are no longer `const`. This is so that the parameters can take any type implementing `Into<CornerRadius>`:

```rust
let n = BorderRadius::top_right(vh(10.));
let m = BorderRadius::top_right([px(10.), px(20.)]);
```
