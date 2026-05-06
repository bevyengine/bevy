---
title: Partial Bindless on Metal and Reduced Bind Group Overhead
authors: ["@holg"]
pull_requests: [23436]
---

Bindless rendering now works on Metal (macOS and iOS) for materials that only use `#[data(...)]`, textures, and samplers — which includes `StandardMaterial`.
This was previously disabled entirely on Metal, because Bevy required both `TEXTURE_BINDING_ARRAY` and `BUFFER_BINDING_ARRAY` support, and Metal only provides the former.
Materials that use uniform buffer binding arrays (via `#[uniform(..., binding_array(...))]`) still correctly fall back to non-bindless on Metal.

We've thrown in two related fixes: The sampler limit check was using the wrong metric — it was checking `max_samplers_per_shader_stage` (binding slot count) rather than `max_binding_array_sampler_elements_per_shader_stage` (array element count). And `create_bindless_bind_group_layout_entries` now only creates binding arrays for resource types a material actually uses. This keeps Metal within its 31 argument buffer slot limit, and reduces wasted bind group overhead on all platforms.

Benchmarked on Bistro Exterior (698 materials), 5-minute runs:

| GPU                      | Avg FPS improvement | Min FPS improvement | Memory      |
| ------------------------ | ------------------- | ------------------- | ----------- |
| Apple M2 Max (Metal)     | +18%                | +77%                | −57 MB RAM  |
| NVIDIA 5060 Ti           | +84%                | +174%               | Same        |
| Intel i360P              | +15%                | Same                | Same        |
| AMD Vega 8 / Ryzen 4800U | Same                | Same                | −88 MB VRAM |
| Intel Iris XE            | Same                | Same                | Same        |
