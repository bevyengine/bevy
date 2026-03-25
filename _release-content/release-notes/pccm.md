---
title: Parallax Corrected Cubemaps
authors: ["@pcwalton"]
pull_requests: [22582]
---

Bevy previously didn't ever apply parallax correction to cubemaps, so reflections were rendered as though the environment were infinitely far away. This is often acceptable for outdoor scenes in which the environment is very distant, but for indoor scenes and dense environments this is undesirable. The standard solution for this problem is parallax correction, in which each reflection probe is augmented with a bounding box, and a raytrace is performed against the bounding box in order to determine the proper direction for sampling the cubemap.

This commit implements parallax correction in Bevy for light probes in an opt-out manner. (You may add the `NoParallaxCorrection` component to a `LightProbe` with an `EnvironmentMapLight` in order to opt out of it.) The bounding box used for parallax correction is assumed to be identical to the bounding box of the influence of the reflection probe itself. This is a reasonable default and matches what Blender does; it's what you want when you have, for example, a cubemap that captures the interior of a rectangular room.

Additionally, a bug was fixed where the transform of each cubemap reflection probe wasn't being taken into account in the shader. I believe that this was being masked because most cubemaps are rendered in world space and therefore most cubemap reflection probes have an identity rotation.

A new example, `pccm`, has been added, demonstrating the effect of parallax correction. It shows a scene consisting of an outer textured cube with an inner reflective cuboid. The outer textured cube contains a reflection probe containing a snapshot of the scene (pre-rendered in Blender). Parallax correction can be toggled on and off in the example in order to demonstrate its effect.
