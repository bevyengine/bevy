---
title: "Immutable Entity Events"
pull_requests: [21408]
---

The mutable methods of `EntityEvent` (`EntityEvent::from` and `EntityEvent::event_target_mut`)
have been moved to a separate trait: `SetEntityEventTarget`

This makes all `EntityEvents` immutable by default.

`SetEntityEventTarget` is implemented automatically for propagated events (e.g. `#[entity_event(propagate)]`).
