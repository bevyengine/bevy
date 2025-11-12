---
title: "Schedule cleanup"
pull_requests: [21608, 21817]
---

- `ScheduleGraph::topsort_graph` has been moved to `DiGraph::toposort`, and now takes a `Vec<N>` parameter for allocation reuse.
- `ReportCycles` was removed: instead, `DiGraphToposortError`s should be immediately
  wrapped into hierarchy graph or dependency graph `ScheduleBuildError` variants.
- `ScheduleBuildError::HierarchyLoop` variant was removed, use `ScheduleBuildError::HierarchySort(DiGraphToposortError::Loop())` instead.
- `ScheduleBuildError::HierarchyCycle` variant was removed, use `ScheduleBuildError::HierarchySort(DiGraphToposortError::Cycle())` instead.
- `ScheduleBuildError::DependencyLoop` variant was removed, use `ScheduleBuildError::DependencySort(DiGraphToposortError::Loop())` instead.
- `ScheduleBuildError::DependencyCycle` variant was removed, use `ScheduleBuildError::DependencySort(DiGraphToposortError::Cycle())` instead.
- `ScheduleBuildError::CrossDependency` now wraps a `DagCrossDependencyError<NodeId>` instead of directly holding two `NodeId`s. Fetch them from the wrapped struct instead.
- `ScheduleBuildError::SetsHaveOrderButIntersect` now wraps a `DagOverlappingGroupError<SystemSetKey>` instead of directly holding two `SystemSetKey`s. Fetch them from the wrapped struct instead.
- `ScheduleBuildError::SystemTypeSetAmbiguity` now wraps a `SystemTypeSetAmbiguityError` instead of directly holding a `SystemSetKey`. Fetch them from the wrapped struct instead.
- `ScheduleBuildWarning::HierarchyRedundancy` now wraps a `DagRedundancyError<NodeId>` instead of directly holding a `Vec<(NodeId, NodeId)>`. Fetch them from the wrapped struct instead.
- `ScheduleBuildWarning::Ambiguity` now wraps a `AmbiguousSystemConflictsWarning` instead of directly holding a `Vec`. Fetch them from the wrapped struct instead.
- `ScheduleGraph::conflicting_systems` now returns a `&ConflictingSystems` instead of a slice. Fetch conflicts from the wrapped struct instead.
- `ScheduleGraph::systems_in_set` now returns a `&HashSet<SystemKey>` instead of a slice, to reduce redundant allocations.
- `ScheduleGraph::conflicts_to_string` functionality has been replaced with `ConflictingSystems::to_string`.
- `ScheduleBuildPass::build` now takes `&mut Dag<SystemKey>` instead of `&mut DiGraph<SystemKey>`, to allow reusing previous toposorts.
- `ScheduleBuildPass::collapse_set` now takes `&HashSet<SystemKey>` instead of a slice, to reduce redundant allocations.
- `simple_cycles_in_component` has been changed from a free function into a method on `DiGraph`.
- `DiGraph::try_into`/`UnGraph::try_into` was renamed to `DiGraph::try_convert`/`UnGraph::try_convert` to prevent overlap with the `TryInto` trait, and now makes use of `TryInto` instead of `TryFrom` for conversions.
