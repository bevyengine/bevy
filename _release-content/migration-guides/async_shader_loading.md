---
title: "Asynchronous `ShaderModule` loading"
pull_requests: [25352]
---

Because getting shader compilation info needs to be asynchronous on wasm and can't be blocking,
`ShaderCache` no longer loads and stores `wgpu::ShaderModule` now, which is moved to inside `PipelineCache`.

Changes:

- `PipelineCache::block_on_render_pipeline` is not available on `wasm32` now.
- `ShaderCache` no longer has `ShaderModule` and `RenderDevice` generic parameters.
- `ShaderCache::get` is changed to return processed shader source code instead of `wgpu::ShaderModule`.
