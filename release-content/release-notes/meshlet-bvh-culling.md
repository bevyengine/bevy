---
title: Virtual Geometry BVH culling
authors: ["@SparkyPotato", "@atlv24"]
pull_requests: [19318]
---

(TODO: Embed example screenshot here)

Bevy's virtual geometry has been greatly optimized with BVH-based culling, leading to almost true scene-complexity invariance on the GPU.

This gets rid of the previous cluster limit that limited the world to 2^24 clusters (about 4 billion triangles).
There are now *no* hardcoded limits to scene size, only unique instance limits due to VRAM usage (since streaming is not yet implemented),
and total instance limits due the current architecture requiring all instances to be uploaded to the GPU every frame.

The screenshot above has 130,000 dragons in the scene, each with about 870,000 triangles, leading to over *115 billion* total triangles in the scene.
However, this still runs at 60 fps on an RTX 4070 at 1440p, with most of the time being due to the instance upload CPU bottleneck mentioned above (taking 14 ms of CPU time).

Speaking of GPU cost, the scene above renders in about 3.5 ms on the 4070, with ~3.1 ms being spent on the geometry render and ~0.4 ms on the material evaluation.
After increasing the instance count to over 1 million (almost *900 billion triangles*!), the GPU time increases to about 4.5 ms, with ~4.1 ms on geometry render and material evaluation remaining constant at ~0.4 ms.
