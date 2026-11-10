---
title: Border Style
authors: ["@Tatsuya0330"]
pull_requests: [25855]
---

The new `BorderStyle` component provides border styles roughly equivalent to CSS's `border-style` property. The selected style is applied to all sides of the UI node. It also follows the rounded shape when used with `BorderRadius`.

The currently implemented keyword values are as follows:

- `solid`
- `double`
- `groove`
- `ridge`
- `inset`
- `outset`

`none` and `hidden` are not supported, because tables are not currently supported.

`dashed` and `dotted` borders are planned for a future implementation.

```rust
(
    Node {
        width: px(80),
        height: px(80),
        border: UiRect::all(px(15)),
        border_radius: BorderRadius::MAX,
        margin: px(20).all(),
        ..default()
    },
    BackgroundColor(MAROON.into()),
    BorderColor::all(SILVER),
    BorderStyle::Ridge,
),
```

See the `borders` example for the visual demonstration.
