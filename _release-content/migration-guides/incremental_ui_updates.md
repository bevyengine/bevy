---
title: "Incremental UI updates"
pull_requests: [25653]
---

- `UiSurface` has been removed.
- The `ui_surface` module has been renamed to `layout_tree`.
- `Measure` trait's receiver is now &self, was &mut self.
- `NodeMeasure`'s receiver is now &self, was &mut self.
- The `bevy_ui::layout::experimental` module has been removed.
- The "ghost_nodes" feature gate has been removed. `GhostNode`s are always enabled.
- `GhostNode`s now require `Node`.
- `GhostNode`s are full UI nodes but given zero size in layout.
- If a node has both `GhostNode` and `FixedNode`, `FixedNode` is ignored.