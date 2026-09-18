---
title: Headless tree view widget
authors: ["@jbuehler23"]
pull_requests: [25821, 25835]
---

`bevy_ui_widgets` now has a headless tree view with no built-in visuals. A `TreeView` root contains nested `TreeItem` rows, and a row's child rows live under a `TreeItemChildren` container inside that row, so the hierarchy is `TreeView -> TreeItem -> TreeItemChildren -> TreeItem`.

`SelectedTreeItem` on the root holds the selected row and the `Expanded` marker component holds the expansion state, so a component hook can keep the accessibility tree in sync. Rows that set `has_children` also get an `Expandable` marker. Interaction emits `ValueChange<Option<Entity>>` and `TreeItemExpandChange` as requests, applied by the app or by the `tree_view_self_update` and `tree_view_expand_self_update` observers. The widget never spawns rows: a row that sets `has_children` with no container yet still emits `TreeItemExpandChange`, so the app can populate it then.

Arrow keys walk the visible rows and expand, collapse, or step in and out of subtrees. Home and End jump to the ends, Enter or Space emits `TreeItemActivate`, and primary-button clicks select a row or, on a `TreeItemToggle`, expand it. Rows derive `Selected` and `TreeItem::level`, carry tree accessibility roles, and honor `InteractionDisabled` per row or on the whole tree.

```rust
(
    TreeView::default()
    SelectedTreeItem(Some(first_row))
    on(tree_view_self_update)
    on(tree_view_expand_self_update)
    Children [
        (TreeItem Children [Text("Camera")]),
        (
            TreeItem { has_children: true }
            Expanded
            Children [
                Text("Scene"),
                (TreeItemChildren Children [(TreeItem Children [Text("Ground")])]),
            ]
        ),
    ]
),
```

See the `headless_tree` example for lazy population, indentation, and keyboard navigation.
