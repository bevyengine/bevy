---
title: Headless split pane widget
authors: ["@jbuehler23"]
pull_requests: [25969]
---

`bevy_ui_widgets` now has a headless split pane with no built-in visuals. A `SplitPane` container lays out its `Pane` children along a horizontal or vertical axis, with `SplitPaneHandle` entities between them. Each pane has a `size`, used as a flex weight, and an optional `min_size` in logical pixels.

Dragging a handle resizes only the two panes next to it, clamped to their minimum sizes. Like other widgets in the crate, the split pane does not change its own state: it emits `ValueChange<Vec<f32>>` from the container with the proposed size of every pane, applied by the app or by the `split_pane_self_update` observer. `InteractionDisabled` on the container blocks dragging.

```rust
bsn! {
    SplitPane { orientation: ControlOrientation::Horizontal }
    on(split_pane_self_update)
    Children [
        Pane { size: 1.0, min_size: 80.0 } Node
        --
        SplitPaneHandle Node { width: px(4) }
        --
        Pane { size: 2.0, min_size: 80.0 } Node
    ]
}
```

See the `headless_split_pane` example for nested horizontal and vertical splits.
