---
title: "`Gizmos::cuboid` has been renamed to `Gizmos::cube`"
pull_requests: [21356]
---

To make way for a function that actually draws cuboids, Bevy 0.17's `Gizmos::cuboid` was renamed to `Gizmos::cube`.

If you were constructing a `Transform` with scale to draw a cuboid before, you can now use `Gizmos::cuboid` directly and supply an Aabb.
