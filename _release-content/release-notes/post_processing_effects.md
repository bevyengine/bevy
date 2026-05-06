---
title: "More Post-Processing Effects"
authors: ["@Breakdown-Dog"]
pull_requests: [22564, 23110]
---

Two new post-processing effects are available for cameras in Bevy 0.19.

## Vignette

*TODO: Add a before/after image showing the vignette effect.*

Vignette reduces image brightness towards the periphery of the frame, drawing the viewer's eye to the center. It's a classic tool for simulating the look of a camera lens or adding cinematic tension.

To use it, add the `Vignette` component to your camera:

```rust
commands.spawn((
    Camera3d::default(),
    Vignette::default(),
));
```

## Lens Distortion

*TODO: Add a before/after image showing the barrel/pincushion warping effect.*

Lens distortion warps the image spatially — pushing the edges outward (barrel distortion) or pulling them inward (pincushion distortion). It's used to simulate real camera optics or to add a stylized, dynamic look to a scene.

To use it, add the `LensDistortion` component to your camera:

```rust
commands.spawn((
    Camera3d::default(),
    LensDistortion::default(),
));
```

