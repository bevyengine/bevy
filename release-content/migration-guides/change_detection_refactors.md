---
title: "Tick-related refactors"
pull_requests: [21562]
---

- `TickCells` is now `ComponentTickCells`.
- `ComponentSparseSet::get_with_ticks` now returns `Option<(Ptr, ComponentTickCells)>` instead of `Option<(Ptr, TickCells, MaybeLocation)>`.
