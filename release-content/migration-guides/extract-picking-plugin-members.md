---
title: Extract `PickingPlugin` members into `PickingSettings`
pull_requests: [19078]
---

Controlling the behavior of picking should be done through
the `PickingSettings` resource instead of `PickingPlugin`.

To initialize `PickingSettings` with non-default values, simply add
the resource to the app using `insert_resource` with the desired value.
