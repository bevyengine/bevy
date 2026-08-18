---
title: "Val::Em and Val::Rem"
authors: ["@gagnus"]
pull_requests: [25231]
---

Bevy UI now supports `em` and `rem` as sizing units. `em` is the current font size (represented by an `EmSize` component), `rem` is a
global "root" font size (represented by the existing `RemSize` resource).

`EmSize` is derived from `TextFont` when one is on the same entity; propagating it down the hierarchy is left to your app.

This is especially useful if you might want to vary your text size after authoring your UIs, for example as an accessibility feature or
just to improve your UI on different devices.

```rust
bsn! {
    Node { width: em(10) }
    Text("Hello")
    TextFont { font_size: FontSize::Rem(1.5) }
}
```

The default font-size is now `rem(1)` rather than `px(20)`. This is a no-op if you're not changing `RemSize` but it means your
text will scale by default when you do.
