---
title: "Schedule cleanup"
pull_requests: [21608]
---

- `ScheduleGraph::topsort_graph` has been moved to `DiGraph::toposort`.
- `ReportCycles` was removed: instead, `DiGraphToposortError`s should be immediately
  wrapped into hierarchy graph or dependency graph `ScheduleBuildError` variants.
- `ScheduleBuildError::HierarchyLoop` variant was removed, use `ScheduleBuildError::HierarchySort(DiGraphToposortError::Loop())` instead.
- `ScheduleBuildError::HierarchyCycle` variant was removed, use `ScheduleBuildError::HierarchySort(DiGraphToposortError::Cycle())` instead.
- `ScheduleBuildError::DependencyLoop` variant was removed, use `ScheduleBuildError::DependencySort(DiGraphToposortError::Loop())` instead.
- `ScheduleBuildError::DependencyCycle` variant was removed, use `ScheduleBuildError::DependencySort(DiGraphToposortError::Cycle())` instead.
- `simple_cycles_in_component` has been changed from a free function into a method on `DiGraph`.
