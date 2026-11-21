---
title: Feathers dock
authors: ["@jbuehler23"]
pull_requests: [26627]
---

A dock can now use feathers styles. Add `FeathersDock` to the entity holding the `DockTree`, and its tab groups, tabs and splitters look like `FeathersTabList`, `FeathersTab` and `FeathersSplitPaneHandle`. While a tab is dragged, the drop zone under the pointer is highlighted, and the tab drag proxy shows when an ancestor has `DragOverlayRoot`.

```rust
commands
    .spawn((FeathersDock, tree, Node { flex_grow: 1.0, ..default() }))
    .observe(dock_self_update);
```

See the `feathers_dock` example.
