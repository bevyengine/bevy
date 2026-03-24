---
title: "Post-process vignette effect"
authors: ["@Breakdown-Dog"]
pull_requests: [22564]
--- 

We’ve added vignette, which is a common post-processing effect that creates a reduction of image brightness towards the periphery compared to the image center. It’s often used to draw focus to the center of the screen or to simulate the look of a camera lens.

To use it, add the `Vignette` component to your camera:

```rust
commands.spawn((
    Camera3d::default(),
    Vignette::default()
))
```
