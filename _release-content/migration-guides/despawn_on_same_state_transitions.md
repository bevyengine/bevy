---
title: "`DespawnOnEnter` / `DespawnOnExit` can now trigger during same state transitions"
pull_requests: [23390]
---

In `bevy_state`, you can define states and transition between them.
For example, this can be used to transition between `AppState::Menu` and `AppState::InGame`, and run different logic depending on the state the `App` is in.
In 0.18, it became possible to transition from a state to itself: A 'same state transition'.
However, there was a bug that made is so that `DespawnOnEnter` and `DespawnOnExit` did not trigger for same state transition when they should have.

In 0.19, this bug is fixed.
If your application transitions between states using `NextState::set()`, your application will trigger `DespawnOnEnter` and `DespawnOnExit`, even in same state transitions.

If this is undesired, use `NextState::set_if_neq()` to transition between states. `set_if_neq()` does not run any state transition schedules if the target state is the same as the current one.
