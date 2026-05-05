---
title: Rectangle Area Lights
authors: ["@dylansechet"]
pull_requests: [23288]
---

*TODO: maybe add image from the PR?*

Bevy's lighting toolkit just got a new addition: rectangular area lights!

The implementation uses [Linearly Transformed Cosines](https://eheitzresearch.wordpress.com/415-2/), which is the standard method for real-time area lights and should also help make our spherical area lights more accurate in the near future.

Rectangular lights currently don't cast shadows or have support for anisotropic materials.

You need to enable the `area_light_luts` cargo feature to use it.

Check out [the new example](https://github.com/bevyengine/bevy/tree/latest/examples/3d/rect_light.rs) to see them in action.
