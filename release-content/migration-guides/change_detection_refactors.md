---
title: "Tick-related refactors"
pull_requests: [21562, 21613]
---

`TickCells` is now `ComponentTickCells`.

`ComponentSparseSet::get_with_ticks` now returns `Option<(Ptr, ComponentTickCells)>` instead of `Option<(Ptr, TickCells, MaybeLocation)>`.

The following types have been moved from the `component` module to the `change_detection` module:

- `Tick`
- `ComponentTicks`
- `ComponentTickCells`
- `CheckChangeTicks`
