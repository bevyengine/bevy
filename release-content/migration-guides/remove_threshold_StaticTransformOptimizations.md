---
title: Feature that broke
pull_requests: [23193]
---

We are removing the threshold all together from StaticTransformOptimizations, so the optimization is either enabled or disabled now.

Don't rely on from_threshold calls, either have the optimizations enabled or disabled. If you want to toggle this dynamically, you can count the entities in a system and dynamically enabled/disable this.
