---
title: "Multi-click support"
authors: ["@ickshonpe"]
pull_requests: [24023, 24264]
---

Bevy Picking now supports multi-click interactions like double-clicks and triple-clicks.

`Pointer<Press>` and `Pointer<Click>` events now include a `count` field, which tracks consecutive presses or clicks on the same entity.
The maximum interval between consecutive presses or clicks can be set using the `PickingSettings` resource.
