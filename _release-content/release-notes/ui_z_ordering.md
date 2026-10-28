---
title: UI z-ordering with ghost nodes
authors: ["@ickshonpe"]
pull_requests: []
---

`GhostNode`s can now take part in UI z-ordering. A `GhostNode` with a `ZIndex` or `GlobalZIndex` stacks its descendants as a group: the group is ordered among the ghost's siblings by its `ZIndex` and placed on its `GlobalZIndex`'s layer, while the descendants' own z-indices order them within the group. A `GhostNode` without either component is skipped, and its children stack as if they belonged to the ghost's parent, so existing layouts keep their order.

This allows root `GhostNode` to act as a container for UI roots that need a fixed order. The children of a root `GhostNode` are UI roots for layout and z-ordering, but their  stacking order is fixed relative to their siblings.
