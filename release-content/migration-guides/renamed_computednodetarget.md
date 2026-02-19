---
title: Renamed `ComputedNodeTarget` and `update_ui_context_system`
pull_requests: [20519, 20532]
---

`ComputedNodeTarget` has been renamed to `ComputedUiTargetCamera`. New name chosen because the component's value is derived from `UiTargetCamera`.

`update_ui_context_system` has been renamed to `propagate_ui_target_cameras`.
