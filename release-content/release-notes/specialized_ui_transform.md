---
title: Specialized UI Transform
authors: ["@Ickshonpe"]
pull_requests: [16615]
---

In Bevy UI `Transform` and `GlobalTransform` have been replaced by `UiTransform` and `UiGlobalTransform`.  `UiTransform` is a specialized 2D UI transform, which more effectively maps to the UI space, improves our internals substantially, and cuts out redundant, unnecessary, often expensive work (such as doing full hierarchical [`Transform`] propagation _in addition_ to the Bevy UI layout algorithm).
