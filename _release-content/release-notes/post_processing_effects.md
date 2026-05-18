---
title: "More Post-Processing Effects"
authors: ["@Breakdown-Dog"]
pull_requests: [22564, 23110]
---

Two new post-processing effects were added in this cycle, both classic tools for giving your camera a more cinematic or stylized look.

## Vignette

*TODO: Add a before/after image showing the vignette effect.*

Vignette reduces image brightness towards the periphery of the frame, drawing the viewer's eye to the center.
It's a classic tool for simulating the look of a camera lens or adding cinematic tension — but its real power in games is as a dynamic effect.
Think pulsing red on damage (a first-person shooter staple), a low uneven dim for horror dread, or a subtle ease-in on cutscene transitions.
The `intensity` of a vignette is a float value; you can change the vignettes effect by animating it.

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

Lens distortion warps the image spatially — positive `intensity` pushes the edges outward (barrel distortion), negative pulls them inward (pincushion distortion).
Racing games ramp up barrel distortion as speed increases, making the world feel like it's bending around the player; push it further and you get a fisheye look, useful for diegetic security cameras, wide-angle surveillance aesthetic or that classic GoPro bodycam look.
Negative values lend themselves to impairment states — drunk, poisoned, or concussed — where you want the world to feel compressed and wrong.

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
