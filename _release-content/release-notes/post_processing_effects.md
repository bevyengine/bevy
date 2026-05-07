---
title: "More Post-Processing Effects"
authors: ["@Breakdown-Dog"]
pull_requests: [22564, 23110]
---

Two new post-processing effects this cycle, both classic tools for giving your camera a more cinematic or stylized look.

## Vignette

*TODO: Add a before/after image showing the vignette effect.*

Vignette reduces image brightness towards the periphery of the frame, drawing the viewer's eye to the center.
It's a classic tool for simulating the look of a camera lens or adding cinematic tension — but it's most powerful in games as a dynamic effect.
Animate `intensity` to pulse a red vignette when the player is hit or critically injured (a staple of first-person shooters).
Keep it dark with an uneven rhythm for persistent unease in a horror game.
Dial it up subtly during cutscene transitions to shift the feel from gameplay to cinema without a hard cut.

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

