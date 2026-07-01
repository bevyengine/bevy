---
title: "Reads and writes from `Access` are exposed"
pull_requests: []
---

Removed from [`bevy_ecs::query::Access`] methods that gave `Result<ComponentIdSet, UnboundedAccessError>>`:

- `try_reads_and_writes()`
- `try_writes()`

Added new functions that return a new enum `InvertibleComponentIdSetRef`:

- `reads_and_writes()`
- `writes()`

The `try_` methods would fail for exclusion queries.
