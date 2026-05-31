---
title: Partial Bindless on Metal and Reduced Bind Group Overhead
authors: ["@holg"]
pull_requests: [23436]
---

In an ideal world, Bevy users could write a single application and ship it everywhere, with every last one of the messy cross-platform differences beautifully abstracted away.
That's a little hard though, and in this particular case, we found that rendering complex scenes on Mac and iOS was markedly slower.

Bindless rendering is how modern engines handle scenes with many different materials efficiently: shaders index into shared pools of textures and buffers rather than rebinding them per draw call.

Metal (Apple's GPU API) has partial bindless support:
it permits texture binding arrays but not buffer binding arrays.
Historically, Bevy required both to enable bindless, which excluded Metal entirely, even for materials that never use buffer arrays.

Most materials, including `StandardMaterial`, do not need buffer array support.
To ensure those materials take the fast path, Bevy now checks the actual needs of each material.
If you only need texture arrays, your material can be rendered efficiently across Bevy's desktop platforms.
If you use `#[uniform(..., binding_array(...))]`, expect unusually poor performance on Metal.

We've also fixed two important correctness bugs in the process.
First, we discovered that the sampler limit check was testing the wrong metric: `max_samplers_per_shader_stage` counts binding slots, but the relevant limit is `max_binding_array_sampler_elements_per_shader_stage`, the array element count (a mismatch that could silently exceed hardware limits).
Second, Bevy now also skips creating binding array slots for resource types a material doesn't use, staying within Metal's hard 31 argument buffer slot limit and reducing overhead on all platforms.

Benchmarked on Bistro Exterior (698 materials), 5-minute runs:

| GPU                      | Avg FPS improvement | Min FPS improvement | Memory      |
| ------------------------ | ------------------- | ------------------- | ----------- |
| Apple M2 Max (Metal)     | +18%                | +77%                | −57 MB RAM  |
| NVIDIA 5060 Ti           | +84%                | +174%               | Same        |
| AMD Vega 8 / Ryzen 4800U | Same                | Same                | −88 MB VRAM |
| Intel i360P              | +15%                | Same                | Same        |
| Intel Iris XE            | Same                | Same                | Same        |

[Bistro] is a demanding, fairly realistic scene.
While Metal's bindless limitations remain frustrating,
it's lovely to see those performance gains, and to know that Bevy is not artificially holding performance on iOS and macOS back.

[Bistro]: https://developer.nvidia.com/orca/amazon-lumberyard-bistro
