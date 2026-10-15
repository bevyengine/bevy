---
title: Redundant mesh view samplers merged
pull_requests: [25836]
---

Several `@group(0)` mesh view bindings each owned an identical `Linear`/`ClampToEdge` sampler.
These are now one shared `linear_sampler`, and the point and directional shadow comparison
samplers are now one. This renumbers the view binding indices.

Rust API:

- `AtmosphereSampler` and `TransmissionSampler` are removed. Use the new `LinearSampler(pub Sampler)` resource (`Res<LinearSampler>`, read `.0`).
- `ViewTransmissionTexture::sampler` is removed. Use `LinearSampler`.
- `ShadowSamplers`' four sampler fields are now a single `shadow_comparison_sampler` field; the removed linear samplers are covered by `LinearSampler`.

- Shaders importing `bevy_pbr::mesh_view_bindings` are unaffected. Shaders that hard-code
`@group(0) @binding(N)` must update: the redundant per-texture samplers (point/directional shadow
linear, transmission, atmosphere, area-light LUT, DFG LUT samplers) are gone, replaced by
`linear_sampler` at binding 22; `shadow_textures_comparison_sampler` (binding 3) is now shared by
point and directional shadows; and every binding after the removed ones shifted down (e.g.
tonemapping LUT `18`/`19` → `15`/`16`).
