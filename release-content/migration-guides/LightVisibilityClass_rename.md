---
title: `LightVisibilityClass` renamed to `ClusterVisibilityClass`
pull_requests: [19986]
---

When clustered decals were added, they used `LightVisibilityClass` to share the clustering infrastructure.
This revealed that this visibility class wasn't really about lights, but about clustering.
It has been renamed to `ClusterVisibilityClass` and moved to live alongside clustering-specific types.
