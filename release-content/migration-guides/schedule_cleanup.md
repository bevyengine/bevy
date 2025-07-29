---
title: Schedule API Cleanup
pull_requests: [19352, 20119, 20172, 20256]
---

In order to support removing systems from schedules, `Vec`s storing `System`s and
`SystemSet`s have been replaced with `SlotMap`s which allow safely removing nodes and
reusing indices. The maps are respectively keyed by `SystemKey`s and `SystemSetKey`s.

The following signatures were changed:

- `DiGraph` and `UnGraph` now have an additional, required type parameter `N`, which
  is a `GraphNodeId`. Use `DiGraph<NodeId>`/`UnGraph<NodeId>` for the equivalent to the previous type.
- `NodeId::System`: Now stores a `SystemKey` instead of a plain `usize`
- `NodeId::Set`: Now stores a `SystemSetKey` instead of a plain `usize`
- `ScheduleBuildPass::collapse_set`: Now takes the type-specific keys.
  Wrap them back into a `NodeId` if necessary.
- `ScheduleBuildPass::build`: Now takes a `DiGraph<SystemKey>` instead of `DiGraph<NodeId>`.
  Re-wrap the keys back into `NodeId` if necessary.
- The following functions now return the type-specific keys. Wrap them back into a `NodeId` if necessary.
  - `Schedule::systems`
  - `ScheduleGraph::conflicting_systems`
- `ScheduleBuildError` variants now contain `NodeId` or type-specific keys, rather than `String`s.
  Use `ScheduleBuildError::to_string` to render the nodes' names and get the old error messages.
- `ScheduleGraph::build_schedule` now returns a `Vec<ScheduleBuildWarning>` in addition to the built
  `SystemSchedule`. Use standard `Result` functions to grab just the `SystemSchedule`, if needed.

The following functions were replaced. Those that took or returned `NodeId` now
take or return `SystemKey` or `SystemSetKey`. Wrap/unwrap them as necessary.

- `ScheduleGraph::contains_set`: Use `ScheduleGraph::system_sets` and `SystemSets::contains`.
- `ScheduleGraph::get_set_at`: Use `ScheduleGraph::system_sets` and `SystemSets::get`.
- `ScheduleGraph::set_at`: Use `ScheduleGraph::system_sets` and `SystemSets::index` (`system_sets[key]`).
- `ScheduleGraph::get_set_conditions_at`: Use `ScheduleGraph::system_sets` and `SystemSets::get_conditions`.
- `ScheduleGraph::system_sets`: Use `ScheduleGraph::system_sets` and `SystemSets::iter`.
- `ScheduleGraph::get_system_at`: Use `ScheduleGraph::systems` and `Systems::get`.
- `ScheduleGraph::system_at`: Use `ScheduleGraph::systems` and `Systems::index` (`systems[key]`).
- `ScheduleGraph::systems`: Use `ScheduleGraph::systems` and `Systems::iter`.

The following enum variants were replaced:

- `ScheduleBuildError::HierarchyRedundancy` with `ScheduleBuildError::Elevated(ScheduleBuildWarning::HierarchyRedundancy)`
- `ScheduleBuildError::Ambiguity` with `ScheduleBuildError::Elevated(ScheduleBuildWarning::Ambiguity)`

The following functions were removed:

- `NodeId::index`: You should match on and use the `SystemKey` and `SystemSetKey` instead.
- `NodeId::cmp`: Use the `PartialOrd` and `Ord` traits instead.
- `ScheduleGraph::set_conditions_at`: If needing to check presence of conditions,
  use `ScheduleGraph::system_sets` and `SystemSets::has_conditions`.
  Otherwise, use `SystemSets::get_conditions`.
