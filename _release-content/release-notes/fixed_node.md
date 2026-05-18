---
title: "FixedNode"
authors: ["@Ickshonpe"]
pull_requests: [24323]
---

`FixedNode` is a new marker component for Bevy UI.

A UI node entity with the `FixedNode` component is positioned relative to the window rather than its parent element.

In the Taffy layout there is nothing to distinguish `FixedNode`s and root nodes, so they are treated identically during updates.
