---
title: Specialized UI Transform
authors: ["@Ickshonpe"]
pull_requests: [16615]
---

In Bevy UI `Transform` and `GlobalTransform` have been replaced by `UiTransform` and `UiGlobalTransform`.  `UiTransform` is a specialized 2D UI transform which supports responsive translations.

This is only the first step in a broader effort to dethrone `Transform` as the do-everything position-rotation-scale type.
Don't worry, we haven't forgotten about 2D: we know that working with quaternions is a headache, and are hoping to define a dedicated 2D transform type, saving space and frustration.
