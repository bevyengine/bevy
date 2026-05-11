---
title: Parallax Corrected Cubemaps
authors: ["@pcwalton"]
pull_requests: [22582]
---

*TODO: Add a before/after screenshot from the `pccm` example showing reflections with and without parallax correction.*

Bevy previously rendered cubemap reflections as though the environment were infinitely far away.
For outdoor scenes this was often fine, but for indoor scenes and dense environments the result looked wrong —
reflections didn't line up with the actual geometry around the viewer.

The standard fix is parallax correction: each reflection probe gets its own bounding box, and a raytrace against that box determines the correct sampling direction for the cubemap.
Bevy now applies this automatically for light probes, using the probe's influence bounding box as the correction volume.
This is a reasonable default for a cubemap capturing a rectangular room interior, and matches Blender's approach.

Parallax correction is enabled by default. To opt out on a specific probe, add `NoParallaxCorrection`:

```rust
commands.spawn((
    LightProbe,
    EnvironmentMapLight { .. },
    NoParallaxCorrection,
));
```

A new `pccm` example demonstrates the effect, with parallax correction toggleable at runtime.
