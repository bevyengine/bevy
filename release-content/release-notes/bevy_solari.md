---
title: Initial raytraced lighting progress (bevy_solari)
authors: ["@JMS55", "@SparkyPotato"]
pull_requests: [19058, 19620, 19790, 20020, 20113, 20156, 20213, 20242, 20259, 20406]
---

(TODO: Embed solari example screenshot here)

In Bevy 0.17, we've made the first steps towards realtime raytraced lighting in the form of the new bevy_solari crate.

For some background, lighting in video games can be split into two parts: direct and indirect lighting.

Direct lighting is light that is emitted from a light source, bounces off of one surface, and then reaches the camera. Indirect lighting by contrast is light that bounces off of different surfaces many times before reaching the camera, and is often called global illumination.

(TODO: Diagrams of direct vs indirect light)

In Bevy, direct lighting comes from analytical light components (`DirectionalLight`, `PointLight`, `SpotLight`) and shadow maps. Indirect lighting comes from a hardcoded `AmbientLight`, baked lighting components (`EnvironmentMapLight`, `IrradianceVolume`, `Lightmap`), and screen-space calculations (`ScreenSpaceAmbientOcclusion`, `ScreenSpaceReflections`, `specular_transmission`, `diffuse_transmission`).

The problem with these methods is that they all have large downsides:

* Emissive meshes do not cast light onto other objects, either direct or indirect.
* Shadow maps are very expensive to render and consume a lot of memory, so you're limited to using only a few shadow casting lights. Good shadow quality can be difficult to obtain in large scenes.
* Baked lighting does not update in realtime as objects and lights move around, is low resolution/quality, and requires time to bake, slowing down game production.
* Screen-space methods have low quality and do not capture off-screen geometry and light.

Bevy Solari is intended as a completely alternate, high-end lighting solution for Bevy that uses GPU-accelerated raytracing to fix all of the above problems. Emissive meshes will properly cast light and shadows, you will be able to have hundreds of shadow casting lights, quality will be much better, it will require no baking time, and it will support _fully_ dynamic scenes!

While Bevy 0.17 adds the bevy_solari crate, it's intended as a long-term project. It is not yet usable by game developers. However, feel free to run the solari example (`cargo run --release --example solari --features bevy_solari` (realtime, no denoising) or `cargo run --release --example solari --features bevy_solari -- --pathtracer` (non-realtime)) to check out the progress we've made, and look forward to more work on Bevy Solari in future releases!

(TODO: Embed bevy_solari logo here, or somewhere else that looks good)

Special thanks to @Vecvec for adding raytracing support to wgpu.
