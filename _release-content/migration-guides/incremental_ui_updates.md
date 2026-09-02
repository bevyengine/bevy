---
title: "Incremental UI updates"
pull_requests: [25653]
---

`UiSurface` has been removed.
The `ui_surface` module has been renamed to `layout_tree`.
`Measure` trait's receiver is now &self, was &mut self.
`NodeMeasure`'s receiver is now &self, was &mut self.
