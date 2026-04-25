---
title: "`DespawnOnEnter` / `DespawnOnExit` can now trigger during same state transitions"
pull_requests: [23390]
---

`DespawnOnEnter` and `DespawnOnExit` can now trigger on entities with those components during same state transitions.

If your application transitions between states using `NextState::set()`, your application will trigger `DespawnOnEnter` and `DespawnOnExit` during same state transitions.

If this is undesired, use `NextState::set_if_neq()` instead to transition between states. `set_if_neq()` does not run any state transition schedules if the target state is the same as the current one.
