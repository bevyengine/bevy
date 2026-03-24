---
title: Partial Bindless on Metal and Reduced Bind Group Overhead
authors: ["@holg"]
pull_requests: [23436]
---

Bindless rendering was previously disabled entirely on Metal (macOS, iOS) because Bevy required both `TEXTURE_BINDING_ARRAY` and `BUFFER_BINDING_ARRAY` support unconditionally. Metal supports the former but not the latter. Since `StandardMaterial` only needs texture and sampler binding arrays - not buffer binding arrays — this requirement was unnecessarily restrictive.

`BUFFER_BINDING_ARRAY` is now only required when a material actually uses buffer binding arrays. Materials that only use `#[data(...)]`, textures, and samplers (including `StandardMaterial`) can now use the bindless path on Metal. A related fix corrects the sampler limit check to use `max_binding_array_sampler_elements_per_shader_stage` (the array element count) instead of `max_samplers_per_shader_stage` (the binding slot count).

Additionally, `create_bindless_bind_group_layout_entries` now only creates binding arrays for resource types the material actually uses, reducing bind group overhead and memory consumption on all platforms.

## Performance

Benchmarked on Bistro Exterior (698 materials), 5-minute runs:

| GPU | Avg FPS improvement | Min FPS improvement | Memory |
| --- | --- | --- | --- |
| Apple M2 Max (Metal) | +18% | +77% | −57 MB RAM |
| NVIDIA 5060 Ti | +84% | +174% | Same |
| Intel i360P | +15% | Same | Same |
| AMD Vega 8 / Ryzen 4800U | Same | Same | −88 MB VRAM |
| Intel Iris XE | Same | Same | No regression |
