---
title: "`DragEnter` now includes the dragged entity"
pull_requests: [19179]
---

`DragEnter` events are now triggered when entering any entity, even the originally dragged one. This makes the behavior more consistent.

The old behavior can be achieved by checking if trigger.entity != trigger.dragged
