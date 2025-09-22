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

When migrating shaders or other custom rendering code, you may encounter panics like:

```raw
wgpu error: Validation Error

Caused by:
  In Device::create_render_pipeline, label = 'pbr_opaque_mesh_pipeline'
    Error matching ShaderStages(FRAGMENT) shader requirements against the pipeline
      Shader global ResourceBinding { group: 2, binding: 100 } is not available in the pipeline layout
        Binding is missing from the pipeline layout


note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
Encountered a panic in system `bevy_render::render_resource::pipeline_cache::PipelineCache::process_pipeline_queue_system`!
```

This error is a result of Bevy's bind group indices changing. Identify the shader by searching for the group and binding mentioned, e.g. `@group(2) @binding(100)`, and follow the above advice to fix the binding group index.
