---
title: "`Val` helper functions"
authors: ["@Ickshonpe", "@TheBlckbird"]
pull_requests: [20518, 20551, 20937]
---

To make `Val`s easier to construct the following helper functions have been added: `px`, `percent`, `vw`, `vh`, `vmin` and `vmax`. Each function takes any integer type and returns the value wrapped by its corresponding `Val` variant. There is also an `auto` helper function that maps to `Val::Auto`.

Also included with this release is a fluent interface for constructing `UiRect`s from `Val`s:

```rust
Node {
    border: px(2).all(), 
    padding: percent(20).horizontal().with_top(px(10.)),
    margin: vw(10).left(),
    ..default()
}
```

The available functions are `left`, `right`, `top`, `bottom`, `all`, `horizontal` and `vertical`.
Each function calls the corresponding `UiRect` constructor on `self`, i.e. `fn left(self) -> UiRect { UiRect::left(self) }`.
