---
title: "More Post-Processing Effects"
authors: ["@Breakdown-Dog"]
pull_requests: [22564, 23110]
---

Two new post-processing effects this cycle, both classic tools for giving your camera a more cinematic or stylized look.

## Vignette

*TODO: Add a before/after image showing the vignette effect.*

Vignette reduces image brightness towards the periphery of the frame, drawing the viewer's eye to the center. It's a classic tool for simulating the look of a camera lens or adding cinematic tension.

To use it, add the `Vignette` component to your camera:

```rust
commands.spawn((
    Camera3d::default(),
    Vignette {
        intensity: 1.0,
        radius: 0.75,
        smoothness: 5.0,
        roundness: 1.0,
        center: Vec2::new(0.5, 0.5),
        edge_compensation: 1.0,
        color: Color::BLACK,
    },
));
```

## Lens Distortion

*TODO: Add a before/after image showing the barrel/pincushion warping effect.*

Lens distortion warps the image spatially — pushing the edges outward (barrel distortion) or pulling them inward (pincushion distortion). It's used to simulate real camera optics or to add a stylized, dynamic look to a scene.

To use it, add the `LensDistortion` component to your camera:

```rust
commands.spawn((
    Camera3d::default(),
    LensDistortion {
        intensity: 0.5,
        scale: 1.0,
        multiplier: Vec2::ONE,
        center: Vec2::splat(0.5),
        edge_curvature: 0.0,
    },
));
```

