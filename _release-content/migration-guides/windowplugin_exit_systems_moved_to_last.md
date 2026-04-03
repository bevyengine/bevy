---
title: "`WindowPlugin` exit systems moved to `Last`"
pull_requests: [23624]
---

`bevy::window::close_when_requested`, `bevy::window::exit_on_all_closed` and `bevy::window::exit_on_primary_closed` have all been moved into the `Last` schedule to prevent systems that run after `Update` and rely on windows existing from panicking on the last frame of the application.

`exit_on_all_closed` and `exit_on_primary_closed` have also been added to a new `SystemSet`, `ExitSystems`.
