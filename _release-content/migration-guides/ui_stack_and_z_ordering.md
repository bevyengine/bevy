---
title: "`UiStack` and `ComputedStackIndex` changes"
pull_requests: [26353]
---
 `UiStack::uinodes` has been removed and `UiStack::partition` is no longer public. Use the new `UiStack::iter` method to iterate over all nodes back-to-front, `UiStack::partitions` to iterate over the partitions, each a back-to-front slice of nodes sharing the same camera target, and `UiStack::len` for the number of nodes.

`ComputedStackIndex`'s field is now private and the component no longer implements `Deref` and `DerefMut`. The accessor `ComputedStackIndex::get` returns the stack index value, which is `None` for nodes that aren't part of the layout (given `ComputedStackIndex::ABSENT`). `GhostNode`s `ComputedStackIndex` is always `ComputedStackIndex::ABSENT`.

`ZIndex` is no longer a required component of `Node`. On a non-ghost node a missing `ZIndex` is still treated as `ZIndex(0)` for ordering. `ZIndex` and `GlobalZIndex` on `GhostNode`s are no longer ignored. Instead a `GhostNode` with a `ZIndex` or `GlobalZIndex` now stacks its descendants as a group.
