---
title: "`StaticTransformOptimizations` no longer stores a threshold for dynamic toggling"
pull_requests: [23193]
---

The threshold has been removed completely from `StaticTransformOptimizations`: the optimization is always either enabled or disabled. As a result this is now a simple `enum`, and some method calls will need to be updated.

If you want to toggle this dynamically, you can count the entities in a system and dynamically enable or disable this.
Performing this check can be slow however, so you probably should not perform this check each frame.
