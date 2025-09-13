---
title: Extract `PointerInputPlugin` members into `PointerInputSettings`
pull_requests: [19078]
---

Toggling mouse and touch input update for picking should be done through
the `PointerInputSettings` resource instead of `PointerInputPlugin`.

To initialize `PointerInputSettings` with non-default values, simply add
the resource to the app using `insert_resource` with the desired value.
