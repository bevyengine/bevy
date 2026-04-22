---
title: Align bevy_transform's feature flags
pull_requests: [23919]
---

`bevy_transform` no longer depends on `bevy_log`. The `bevy_log` feature flag
has been removed.

Tracing instrumentation is now gated on the new `trace` feature (using `tracing`
directly, matching `bevy_ecs`):

```toml
# 0.18
bevy_transform = { features = ["bevy_log"] }

# 0.19
bevy_transform = { features = ["trace"] }
```

Parallel transform propagation is no longer tied to the `std` feature. It now
requires the explicit `multi_threaded` feature:

```toml
# 0.18 — parallel was enabled implicitly via std
bevy_transform = { features = ["std"] }

# 0.19 — opt in explicitly
bevy_transform = { features = ["std", "multi_threaded"] }
```
