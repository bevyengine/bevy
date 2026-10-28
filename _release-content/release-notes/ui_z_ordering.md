---
title: UI z-ordering with ghost nodes
authors: ["@ickshonpe"]
pull_requests: []
---

`GhostNode`s can now take part in UI z-ordering. A `GhostNode` with a `ZIndex` or `GlobalZIndex` stacks its descendants as a group: the group is ordered among the ghost's siblings by its `ZIndex` and placed on its `GlobalZIndex`'s layer, while the descendants' own z-indices order them within the group. A `GhostNode` without either component is skipped, and its children stack as if they belonged to the ghost's parent, so existing layouts keep their order.

This makes a root `GhostNode` a convenient container for UI roots that need a fixed order. Its children are UI roots for layout and are treated as roots for z-ordering too, but their stacking order is fixed by the order of the ghost's `Children` instead of being decided by whichever one changed last.

The rules for ordering nodes on the same `GlobalZIndex` layer are now the same for `ZIndex` and `GlobalZIndex`. Within a hierarchy the order is always hierarchical: `ZIndex` value, then `Children` order, and changing a nested node's z-index never reorders it against its siblings. Between separate hierarchies, all of a hierarchy's nodes on a layer move together: adding, changing or removing a `ZIndex` or `GlobalZIndex` on the root, or a `GlobalZIndex` anywhere in the hierarchy, brings the whole hierarchy to the front of its layer, keeping its internal order. Re-inserting a component with its current value counts as a change, so `commands.entity(window).insert(GlobalZIndex(0))` is all it takes to bring a window to the front.

`UiStack` has also been reworked: `UiStack::partitions` yields per-camera node lists, and `UiStack::iter` walks every node back-to-front.
