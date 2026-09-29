---
title: Feathers split pane
authors: ["@jbuehler23"]
pull_requests: []
---

`bevy_feathers` now has a styled split pane built on the headless `SplitPane` widget. `FeathersSplitPane` is the container, `FeathersPane` is a resizable child, and `FeathersSplitPaneHandle` is drawn as a thin line that highlights while hovered or dragged and shows a resize cursor matching the split direction.

As with the headless widget, pane sizes are only updated when `split_pane_self_update` or your own observer handles the `ValueChange<Vec<f32>>` event.

```rust
bsn! {
    @FeathersSplitPane { @orientation: ControlOrientation::Horizontal }
    on(split_pane_self_update)
    Children [
        @FeathersPane { @min_size: 80.0 }
        --
        @FeathersSplitPaneHandle
        --
        @FeathersPane { @size: 2.0, @min_size: 80.0 }
    ]
}
```

See the `feathers_gallery` example for a horizontal split with a nested vertical split.
