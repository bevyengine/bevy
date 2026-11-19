---
title: Schedule Build Pass API changes
pull_requests: [26388]
---

`ScheduleBuildPass::collapse_set` now also receives the `World` and the `ScheduleGraph`, as
`build` does, so that a pass can configure systems and set up state as it collapses a set.

`SystemSets::get`, `SystemSets::iter` and indexing `SystemSets` now give `InternedSystemSet`
instead of `&dyn SystemSet`, which passes can store and compare cheaply.

## Migration Steps

If you have custom implementations of `ScheduleBuildPass`, add the `world: &mut World` and
`graph: &mut ScheduleGraph` parameters to `collapse_set`, after `systems`.

Where you used the `&dyn SystemSet` that `SystemSets` gave, dereference the `InternedSystemSet`.
