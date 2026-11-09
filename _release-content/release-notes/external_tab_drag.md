---
title: Drag tabs between tab lists
authors: ["@jbuehler23"]
pull_requests: [25985]
---

Headless tabs in `bevy_ui_widgets` can now be dragged from one tab list to another. Set `TabList::drag` to `TabDragMode::External` on each list that should share tabs. An `External` list still supports reordering within itself, while a `Reorder` list never accepts tabs from other lists and its tabs never leave it.

While a tab is dragged over a compatible list, that list carries the `TabInsertionPreview`. Unlike a `Reorder` drag, which keeps tracking its own list wherever the pointer goes, an `External` drag over no compatible list shows no preview. Releasing over it emits `TabMoved` on the source list with `to_strip` set to the destination. The widget does not change the hierarchy, so the same observer used for reordering also applies cross-list moves:

```rust
fn apply_tab_move(moved: On<TabMoved>, mut commands: Commands) {
    commands
        .entity(moved.to_strip)
        .insert_child(moved.index, moved.tab);
}
```

`TabDragLifecycle` is triggered on the source list when a drag starts, completes, or is cancelled. A completed drag reports its `TabDrop`, or `None` when the tab was released outside any compatible list, so apps can decide what such a drop means.
