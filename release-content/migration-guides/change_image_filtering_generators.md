---
title: Changes to image sampler generators
pull_requests: [19127]
---

With the addition of bilinear and anisotropic presets, trilinear is a more suited name.

- `ImageSamplerDescriptor::linear()` is now `ImageSamplerDescriptor::trilinear()`.
- `ImageSampler::linear()` is now `ImageSampler::trilinear()`.
- `ImagePlugin::default_linear()` is now `ImagePlugin::default_trilinear()`.

To stay consistent with the rest of the engine, `ImageSamplerDescriptor::default()`
now returns `ImageSamplerDescriptor::trilinear()`.
Replace it with `ImageSamplerDescriptor::nearest()` to replicate previous behavior.
