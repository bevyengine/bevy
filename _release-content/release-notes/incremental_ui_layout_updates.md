---
title: Incremental UI layout updates
authors: ["@ickshonpe"]
pull_requests: [25653]
---

UI layout has been reimplemented so that updates are more incremental. Instead of storing a duplicate mirror `TaffyTree` in `UiSurface`, the new `UiLayoutTree` adaptor allows `Taffy` to access the ECS component data directly. The layout geometry in `ComputedNode` is now updated only in response to changes in the input layout data.
