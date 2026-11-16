---
title: "`ui_layout_system` has been split up."
pull_requests: [25653]
---

`ui_layout_system` has been split into `update_ui_roots`, `sync_taffy_styles_with_nodes`, `mark_dirty_ui_trees`, `ui_layout_system`, `update_computed_nodes` and `update_border_radius`.

Code that was ordered against `ui_layout_system` should be ordered against the `UiSystems::Layout` system set instead.
