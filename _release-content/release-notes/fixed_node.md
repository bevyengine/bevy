---
title: "FixedNode"
authors: ["@Ickshonpe"]
pull_requests: [24323]
---

`FixedNode` is a new marker component for Bevy UI.

A UI node entity with the `FixedNode` component is positioned relative to the target camera's viewport rather than its parent element. `FixedNode`s don't inherit their parent's layout, clipping or transform context.

In the Taffy layout (stored in `UiSurface`) there is nothing to distinguish `FixedNode`s and root nodes, so they are treated identically during updates.
