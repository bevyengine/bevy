---
title: "Removed the `UiSurface` resource and `ui_surface` module."
pull_requests: [25653]
---

 Instead of maintaining a separate `TaffyTree` in the `UiSurface` resource, `bevy_ui` now has a new `UiLayoutTree` adapter that allows Taffy to access the ECS component data directly. As a consequence, `UiSurface` is no longer needed and it has been removed, along with the `ui_surface` module.
