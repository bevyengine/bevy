---
title: "`wgpu` 25"
pull_requests: [ 19563 ]
---

`wgpu` 25 introduces a number of breaking changes, most notably in the way Bevy is required to handle
uniforms with dynamic offsets which are used pervasively in the renderer. Dynamic offsets and uniforms
of any kind are no longer allowed to be used in the same bind group as binding arrays. As such, the
following changes to the default bind group numbering have been made in 3d:

- `@group(0)` view binding resources
- `@group(1)` view resources requiring binding arrays
- `@group(2)` mesh binding resources
- `@group(3)` material binding resources

Most users who are not using mid-level render APIs will simply need to switch their material bind groups
from `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})`. The `MATERIAL_BIND_GROUP` shader def has been added
to ensure backwards compatibility in the event the bind group numbering changes again in the future.

Exported float constants from shaders without an explicit type declaration like `const FOO = 1.0;` are no
longer supported and must be explicitly typed like `const FOO: f32 = 1.0;`.

See the [full changelog here](https://github.com/gfx-rs/wgpu/blob/trunk/CHANGELOG.md#v2500-2025-04-10).

When migrating custom shaders, you may encounter panics like:

```raw
2025-09-14T14:40:30.518334Z ERROR wgpu::backend::wgpu_core: Handling wgpu errors as fatal by default

thread 'Async Compute Task Pool (0)' (945450) panicked at /home/hhh/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-26.0.1/src/backend/wgpu_core.rs:1327:26:
wgpu error: Validation Error

Caused by:
  In Device::create_render_pipeline, label = 'pbr_opaque_mesh_pipeline'
    Error matching ShaderStages(FRAGMENT) shader requirements against the pipeline
      Shader global ResourceBinding { group: 2, binding: 100 } is not available in the pipeline layout
        Binding is missing from the pipeline layout


note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
Encountered a panic in system `bevy_render::render_resource::pipeline_cache::PipelineCache::process_pipeline_queue_system`!
```

Follow the advice above and fix your bindings to fix these problems.
