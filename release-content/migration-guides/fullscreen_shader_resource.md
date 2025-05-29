---
title: `FULLSCREEN_SHADER_HANDLE` replaced with `FullscreenShader`
pull_requests: [19426]
---

`FULLSCREEN_SHADER_HANDLE` and `fullscreen_shader_vertex_state` have been replaced by the
`FullscreenShader` resource. Users of either of these will need to call `FullscreenShader::shader`
or `FullscreenShader::to_vertex_state` respectively. You may need to clone `FullscreenShader` out of
the render world to store an instance that you can use later (e.g., if you are attempting to use the
fullscreen shader inside a `SpecializedRenderPipeline` implementation).
