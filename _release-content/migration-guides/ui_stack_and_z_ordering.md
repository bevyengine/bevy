---
title: "`UiStack` and UI z-ordering changes"
pull_requests: []
---

`UiStack::uinodes` has been removed and `UiStack::partition` is now a `Vec<Vec<Entity>>` instead of a `Vec<Range<usize>>`. Each inner `Vec` holds the nodes of one partition, back-to-front. Use the new `UiStack::iter` method to iterate over all nodes back-to-front, or index `partition` directly:

```rust
// 0.19
for entity in ui_stack.uinodes.iter() { /* ... */ }
for range in ui_stack.partition.iter() {
    let nodes = &ui_stack.uinodes[range.clone()];
}

// 0.20
for entity in ui_stack.iter() { /* ... */ }
for nodes in ui_stack.partition.iter() { /* ... */ }
```

`ZIndex` is no longer a required component of `Node`. A missing `ZIndex` is still treated as `ZIndex(0)` for ordering, but on a `GhostNode` the presence of a `ZIndex` or `GlobalZIndex` now has a meaning of its own (see below), so `Node` can't insert one by default. Queries such as `Query<&ZIndex, With<Node>>` or `Query<&mut ZIndex>` no longer match nodes that didn't insert a `ZIndex` explicitly. Use `Option<&ZIndex>`, or insert the component on the nodes you need to query.

`GhostNode`s are no longer added to the `UiStack` and their `ComputedStackIndex` is not updated.

The rules that order UI nodes on the same `GlobalZIndex` layer have changed:

- A `GhostNode` with a `ZIndex` or `GlobalZIndex` now stacks its descendants as a group, ordered among the ghost's siblings by its `ZIndex` and placed on its `GlobalZIndex`'s layer. Previously the `ZIndex` and `GlobalZIndex` of a `GhostNode` were ignored. A `GhostNode` without either component is skipped during z-ordering as before.
- The order between separate root hierarchies on the same layer is now decided per hierarchy rather than per node. A nested node with a `GlobalZIndex` no longer uses its own `ZIndex`, or its own change state, to break ties against other hierarchies; it takes its root's. All of a hierarchy's nodes on a layer move together and keep their hierarchical order.
- A hierarchy is brought to the front of its layer when a `ZIndex` or `GlobalZIndex` is added to, changed on or removed from its root, or when a `GlobalZIndex` is added to, changed on or removed from any of its nodes. Removing a component now counts as a change; previously only insertions and mutations did.
- The children of a root `GhostNode` are treated as roots: a `ZIndex` change on one of them, or a new child, brings the whole group to the front of its layer.
