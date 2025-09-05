---
title: Virtual Geometry BVH culling
authors: ["@SparkyPotato", "@atlv24"]
pull_requests: [19318]
---

(TODO: Embed example screenshot here)

Bevy's virtual geometry has been greatly optimized with BVH-based culling, making the cost of rendering nearly independent of scene geometry.
Comparing the sample scene shown above with 130k dragon instances to one with over 1 million instances, total GPU rendering time only increases by 30%.

This also gets rid of the previous cluster limit that limited the world to 2^24 clusters (about 4 billion triangles).
There are now *no* hardcoded limits to scene size. In practice you will only be limited by asset VRAM usage (since streaming is not yet implemented),
and total instance count due the current code requiring all instances to be re-uploaded to the GPU every frame.

The screenshot above has 130,000 dragons in the scene, each with about 870,000 triangles, leading to over *115 billion* total triangles in the scene.

Speaking of concrete GPU cost, the scene above renders in about 3.5 ms on the 4070, with \~3.1 ms being spent on the geometry render and \~0.4 ms on the material evaluation.
After increasing the instance count to over 1 million (almost *900 billion triangles*!), the total increases to about 4.5 ms, with \~4.1 ms on geometry render and material evaluation remaining constant at ~0.4 ms.
This is a 30% increase in GPU time for an almost 8x increase in scene complexity.

Comparing GPU times to 0.16 on a much smaller scene with 1,300 instances, previously the full render took 2.2 ms, whereas now it is 1.3 ms.
