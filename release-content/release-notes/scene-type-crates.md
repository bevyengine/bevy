---
title: Define scenes without depending on bevy_render
authors: ["@atlv24", "@Ickshonpe", "@zeophlite"]
pull_requests: [20485, 20330, 18703, 20587, 20502, 19997, 19991, 20000, 19949, 19943, 19953, 20498, 20496, 20493, 20492, 20491, 20488, 20487, 20486, 20483, 20480, 20479, 20478, 20477, 20473, 20472, 20471, 20470, 20392, 20390, 20388, 20345, 20344, 20051, 19985, 19973, 19965, 19963, 19962, 19960, 19959, 19958, 19957, 19956, 19955, 19954, 16620, 16619, 15700, 15666, 15650]
---

In **Bevy 0.17** we have decoupled most of the user-facing renderer API from `bevy_render` (Bevy's default built-in renderer, which uses [`wgpu`](https://github.com/gfx-rs/wgpu)). It is now possible to use cameras, lights, shaders, images, meshes, sprites, text, ui, picking, animation, and scenes without depending on `bevy_render`.

With these changes, it is now possible for 3rd party custom renderers to act as drop-in replacements for rendering Bevy scenes, without the need to pull in `bevy_render`.

This is also incredibly important for reducing compile time, especially for 3rd party crates: crate authors can now depend more granularly on the specific crates they need. If they don't need access to renderer internals, they don't need to wait for them to start compiling! This increases the potential for parallel compilation.

Additionally, "shader library only" crates with minimal dependencies are now possible thanks to the new separate `bevy_shader` crate.
