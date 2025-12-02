---
title: Replaced `Column` with `ThinColumn`
pull_requests: [21427]
---

The low-level `Column` and `ThinColumn` types in `bevy_ecs` have been
merged into a single type, now called `Column` but with the api
of `ThinColumn`. This type does not keep track of its own allocated
length, and only provides unsafe methods.
