---
title: Dock tree
authors: ["@jbuehler23"]
pull_requests: []
---

The new `bevy_ui_dock` crate describes dockable layouts as plain data. A `DockTree` component holds tab groups and the splits between them, with operations to add, move, split and close tabs. Empty groups are removed and nested splits are merged as the tree changes.

Split sizes are flex weights, the same as `Pane::size` in the split pane widget. Each tab names its content with a `PanelKey` that the app maps to its own UI, and with the `serialize` feature a layout can be saved and restored.

```rust
let mut tree = DockTree::new();
let center = tree.root();
tree.add_tab(center, "viewport")?;
tree.split(center, Edge::Left, "outliner")?;
tree.split(center, Edge::Bottom, "assets")?;
```

This crate holds the data only. Building UI from the tree and dragging tabs between groups come in later releases.
