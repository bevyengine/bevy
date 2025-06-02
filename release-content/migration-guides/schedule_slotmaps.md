---
title: Schedule SlotMaps
pull_requests: [19352]
---

In order to support removing systems from schedules, `Vec`s storing `System`s and
`SystemSet`s have been replaced with `SlotMap`s which allow safely removing and
reusing indices. The maps are respectively keyed by `SystemKey`s and `SystemSetKey`s.

The following signatures were changed:

- `NodeId::System`: Now stores a `SystemKey` instead of a plain `usize`
- `NodeId::Set`: Now stores a `SystemSetKey` instead of a plain `usize`
- `ScheduleBuildPass::collapse_set`: Now takes the type-specific keys. Wrap them back into a `NodeId` if necessary.
- The following functions now return the type-specific keys. Wrap them back into a `NodeId` if necessary.
  - `Schedule::systems`
  - `ScheduleGraph::systems`
  - `ScheduleGraph::system_sets`
  - `ScheduleGraph::conflicting_systems`
- Use the appropriate key types to index these structures rather than bare `usize`s:
  - `ScheduleGraph::systems` field
  - `ScheduleGraph::system_conditions`
- The following functions now take the type-specific keys. Use pattern matching to extract them from `NodeId`s, if necessary:
  - `ScheduleGraph::get_system_at`
  - `ScheduleGraph::system_at`
  - `ScheduleGraph::get_set_at`
  - `ScheduleGraph::set_at`
  - `ScheduleGraph::get_set_conditions_at`
  - `ScheduleGraph::set_conditions_at`

The following functions were removed:

- `NodeId::index`: You should match on and use the `SystemKey` and `SystemSetKey` instead.
- `NodeId::cmp`: Use the `PartialOrd` and `Ord` traits instead.
