---
title: Initial raytraced lighting progress (bevy_solari)
authors: ["@JMS55"]
pull_requests: [19058]
---

(TODO: Embed solari example screenshot here)

In Bevy 0.17, we've made the first steps towards realtime raytraced lighting in the form of the new bevy_solari crate.

For some background, lighting in video games can be split into two parts: direct and indirect lighting.

Direct lighting is light that that is emitted from a light source, bounces off of one surface, and then reaches the camera. Indirect lighting by contrast is light that bounces off of different surfaces many times before reaching the camera, and is often called global illumination.

(TODO: Diagrams of direct vs indirect light)

In Bevy, direct lighting comes from analytical light components (`DirectionalLight`, `PointLight`, `SpotLight`) and shadow maps. Indirect lighting comes from a hardcoded `AmbientLight`, baked lighting components (`EnvironmentMapLight`, `IrradianceVolume`, `Lightmap`), and screen-space calculations (`ScreenSpaceAmbientOcclusion`, `ScreenSpaceReflections`, `specular_transmission`, `diffuse_transmission`).

The problem with these methods is that they all have large downsides:
* Emissive meshes do not cast light onto other objects, either direct or indirect.
* Shadow maps are very expensive to render and consume a lot of memory, so you're limited to using only a few shadow casting lights. Good quality can be difficult to obtain in large scenes.
* Baked lighting does not update in realtime as objects and lights move around, is low resolution/quality, and requires time to bake, slowing down game production.
* Screen-space methods have low quality and do not capture off-screen geometry and light.

Bevy Solari is intended as a completely alternate, high-end lighting solution for Bevy that uses GPU-accelerated raytracing to fix all of the above problems. Emissive meshes will properly cast light and shadows, you will be able to have hundreds of shadow casting lights, quality will be much better, it will require no baking time, and it will support _fully_ dynamic scenes!

While Bevy 0.17 adds the bevy_solari crate, it's intended as a long-term project. Currently there is only a non-realtime path tracer intended as a reference and testbed for developing Bevy Solari. There is nothing usable yet for game developers. However, feel free to run the solari example to see the path tracer is action, and look forwards to more work on Bevy Solari in future releases! (TODO: Is this burying the lede?)

(TODO: Embed bevy_solari logo here, or somewhere else that looks good)

Special thanks to @Vecvec for adding raytracing support to wgpu.

## Goals

Answer the following:

- What has been changed or added?
- Why is this a big deal for users?
- How can they use it?

## Style Guide

You may use markdown headings levels two and three, and must not start with a heading. Prose is appreciated, but bullet points are acceptable. Copying the introduction from your PR is often a good place to start.
