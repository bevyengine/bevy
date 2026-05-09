---
title: Untie Pbr library methods from StandardMaterial textures
authors: ["@hukasu"]
pull_requests: [21714]
---

Bevy's Pbr has multiple shader libraries that can be imported by other shaders,
but many of these methods expect the textures to be available on the bindings
set by `StandardMaterial`.

The following methods have been modified to take the textures as arguments
so that they are no longer tied to `StandardMaterial`:

* `bevy_pbr::parallax_mapping::parallaxed_uv`
* `bevy_pbr::parallax_mapping::sample_depth_map`
