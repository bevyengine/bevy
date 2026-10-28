---
title: "`UiStack` changes"
pull_requests: []
---

`UiStack`'s fields are now private: `UiStack::uinodes` has been removed and `UiStack::partition` is no longer public. Use the new `UiStack::iter` method to iterate over all nodes back-to-front, `UiStack::partitions` to iterate over the partitions, each a back-to-front slice of nodes sharing the same camera target, and `UiStack::len` for the number of nodes:

```rust
// 0.19
for entity in ui_stack.uinodes.iter() { /* ... */ }
for range in ui_stack.partition.iter() {
    let nodes = &ui_stack.uinodes[range.clone()];
}

// 0.20
for entity in ui_stack.iter() { /* ... */ }
for nodes in ui_stack.partitions() { /* ... */ }
```

`ZIndex` is no longer a required component of `Node`. A missing `ZIndex` is still treated as `ZIndex(0)` for ordering, but on a `GhostNode` the presence of a `ZIndex` or `GlobalZIndex` now has a meaning of its own (see below), so `Node` can't insert one by default. Queries such as `Query<&ZIndex, With<Node>>` or `Query<&mut ZIndex>` no longer match nodes that didn't insert a `ZIndex` explicitly. Use `Option<&ZIndex>`, or insert the component on the nodes you need to query.

`ComputedStackIndex`'s field is now private and the component no longer implements `Deref` and `DerefMut`. Use `ComputedStackIndex::get`, which returns `None` for nodes that aren't part of the `UiStack`. Those nodes are `ComputedStackIndex::HIDDEN`, meaning they are neither displayed nor interactable, which is also the new `Default`. `GhostNode`s are no longer added to the `UiStack`, so their `ComputedStackIndex` is always `ComputedStackIndex::HIDDEN`.

The rules that order UI nodes on the same `GlobalZIndex` layer have changed:

- A `GhostNode` with a `ZIndex` or `GlobalZIndex` now stacks its descendants as a group, ordered among the ghost's siblings by its `ZIndex` and placed on its `GlobalZIndex`'s layer. Previously the `ZIndex` and `GlobalZIndex` of a `GhostNode` were ignored. A `GhostNode` without either component is skipped during z-ordering as before.
- The order between separate root hierarchies on the same layer is now decided per hierarchy rather than per node. A nested node with a `GlobalZIndex` no longer uses its own `ZIndex`, or its own change state, to break ties against other hierarchies; it takes its root's. All of a hierarchy's nodes on a layer move together and keep their hierarchical order.
- A hierarchy is brought to the front of its layer when a `ZIndex` or `GlobalZIndex` is added to, changed on or removed from its root, or when a `GlobalZIndex` is added to, changed on or removed from any of its nodes. Removing a component now counts as a change; previously only insertions and mutations did.
- The children of a root `GhostNode` are treated as roots: a `ZIndex` change on one of them, or a new child, brings the whole group to the front of its layer.
