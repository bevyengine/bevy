---
title: Drag to reorder tabs
authors: ["@jbuehler23"]
pull_requests: [25984]
---

Headless tabs in `bevy_ui_widgets` can now be reordered by dragging. Set `TabList::drag` to `TabDragMode::Reorder` to opt in; the default is `TabDragMode::Disabled`. Add `TabLocked` to a tab to keep it focusable and selectable but not draggable.

A drag starts once the pointer moves past a small threshold. The dragged tab then carries `TabDragging`, and the list carries a `TabInsertionPreview` for each active pointer. The insertion point follows the pointer along the list's axis, even when it leaves the list. Each entry has an `index` counted after removing the dragged tab, and a `slot` counted among all tabs, so styling can mark the side of the dragged tab nearest the pointer. Escape, pointer cancellation, or despawning the tab cancels the drag, and releasing a dragged tab does not select it.

As with selection, the widget never changes the hierarchy itself. Releasing emits `TabMoved` with the insertion index, and the app applies it:

```rust
fn apply_tab_move(moved: On<TabMoved>, mut commands: Commands) {
    commands
        .entity(moved.to_strip)
        .insert_child(moved.index, moved.tab);
}
```
