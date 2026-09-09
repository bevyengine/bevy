---
title: Drag proxies for UI widgets
authors: ["@jbuehler23"]
pull_requests: []
---

`bevy_ui_widgets` now has a small drag-proxy facility for widgets that show a visual following the pointer during a drag.

The initiating widget spawns whatever visual it wants as a descendant of the dragged entity and attaches `DragProxy { pointer_id, offset }`. `DragProxyPlugin` then reparents the visual to the nearest ancestor marked with `DragOverlayRoot`, makes it unpickable, keeps it positioned under the pointer in logical UI pixels, and despawns it when the drag ends, is cancelled, the drag source disappears, or Escape is pressed.

```rust
commands.spawn((
    Node::default(),
    ChildOf(dragged_tab),
    DragProxy {
        pointer_id: event.pointer.id,
        offset: Vec2::new(12.0, 12.0),
    },
    children![Text::new("Outline")],
));
```

This is the second step of the tab and docking work from the widgets roadmap. Draggable tabs, which use it, follow in a separate PR.
