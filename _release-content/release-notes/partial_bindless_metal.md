---
title: Partial Bindless on Metal and Reduced Bind Group Overhead
authors: ["@holg"]
pull_requests: [23436]
---

Cross-platform game engines must constantly navigate real differences in platform APIs.
Bevy's goal is to let users write a single application and ship it everywhere —
Windows, Mac, Linux, mobile — with confidence that it will just work.
That's a tough promise to live up to: rendering complex scenes on Mac and iOS was markedly slower.

Bindless rendering is how modern engines handle scenes with many different materials efficiently: shaders index into shared pools of textures and buffers rather than rebinding them per draw call.
Bindless is not just a performance optimization — it's how modern renderers are structured.

Metal (Apple's GPU API) supports texture binding arrays but not buffer binding arrays.
Bevy required both to enable bindless, which previously excluded Metal entirely — even for materials that never use buffer arrays.
If you were shipping on Mac or iOS, your game was running on a slower, fundamentally different code path.

Most materials, including `StandardMaterial`, only use `#[data(...)]`, textures, and samplers — they never needed buffer array support.
Bevy now checks what each material actually needs;
if it only needs texture arrays, it gets bindless on Metal.
Materials using `#[uniform(..., binding_array(...))]` still fall back to non-bindless on Metal.

Two correctness bugs were fixed in the process.
The sampler limit check was testing the wrong metric: `max_samplers_per_shader_stage` counts binding slots, but the relevant limit is `max_binding_array_sampler_elements_per_shader_stage`, the array element count — a mismatch that could silently exceed hardware limits.
Bevy now also skips creating binding array slots for resource types a material doesn't use, staying within Metal's hard 31 argument buffer slot limit and reducing overhead on all platforms.

Benchmarked on Bistro Exterior (698 materials), 5-minute runs:

| GPU                      | Avg FPS improvement | Min FPS improvement | Memory      |
| ------------------------ | ------------------- | ------------------- | ----------- |
| Apple M2 Max (Metal)     | +18%                | +77%                | −57 MB RAM  |
| NVIDIA 5060 Ti           | +84%                | +174%               | Same        |
| Intel i360P              | +15%                | Same                | Same        |
| AMD Vega 8 / Ryzen 4800U | Same                | Same                | −88 MB VRAM |
| Intel Iris XE            | Same                | Same                | Same        |
