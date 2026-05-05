---
title: "Lens Distortion"
authors: ["@Breakdown-Dog"]
pull_requests: [23110]
--- 

We’ve added lens distortion, which is a common post-processing effect that creates a spatial warping of the image towards the periphery compared to the image center. It’s often used to simulate the optical characteristics of a real camera lens or to add a stylized, dynamic look to the scene.

To use it, add the `LensDistortion` component to your camera:

```rust
commands.spawn((
    Camera3d::default(),
    LensDistortion::default(),
))
```
