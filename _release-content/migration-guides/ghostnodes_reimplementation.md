---
title: "`GhostNode` reimplementation"
pull_requests: [25653]
---

The `ghost_nodes` feature gate has been removed. The `experimental` module and its `ghost_nodes` submodule have been removed.

`GhostNode`s are now always enabled, require `Node` and act more like regular UI nodes. The `UiChildren` and `UiRoots` system parameters have been removed.

During layout updates a `GhostNode` is replaced by its children recursively, so that its nearest non-ghost descendants become children of its nearest non-ghost ancestor. The `GhostNode` entity itself is set to zero-size.`UiTransform` is propagated through `GhostNode`s normally, except that for percentage translations the closest non-ghost ancestor's base size is used. Otherwise, since GhostNodes always have zero-size, percentage translations would always resolve to zero.

If a UI node has both the `GhostNode` and `FixedNode` components, `FixedNode` is ignored.
