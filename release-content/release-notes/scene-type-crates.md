---
title: Define scenes without depending on bevy_render
authors: ["@atlv24", "@Ickshonpe", "@zeophlite"]
pull_requests: [20485, 20330, 18703, 20587, 20502, 19997, 19991, 20000, 19949, 19943, 19953, 20498, 20496, 20493, 20492, 20491, 20488, 20487, 20486, 20483, 20480, 20479, 20478, 20477, 20473, 20472, 20471, 20470, 20392, 20390, 20388, 20345, 20344, 20051, 19985, 19973, 19965, 19963, 19962, 19960, 19959, 19958, 19957, 19956, 19955, 19954, 16620, 16619, 15700, 15666, 15650]
---

It is now possible to use cameras, lights, shaders, images, meshes, sprites, text, ui, picking, animation, and scenes without depending on the Bevy renderer. This makes it possible for 3rd party custom renderers to be drop-in replacements for rendering existing scenes.

This is incredibly important for reducing compile time, especially for 3rd party crates: crate authors can now depend more granularly on the specific crates they need, meaning greater chances for compilation parallelism emerge as not everything is bottlenecked on waiting for the bevy_render -> bevy_core_pipelines -> bevy_pbr/bevy_sprite chain to compile.

Another sweet side-effect is that "shader library only" crates are now possible with minimal dependencies thanks to bevy_shader.
