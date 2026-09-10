---
title: "`DownsampleShaders::general` is now a `Handle<Shader>`"
pull_requests: [25744]
---

`DownsampleShaders::general` is now a single `Handle<Shader>` instead of a `HashMap<TextureFormat, Handle<Shader>>`.

Code that looked up a shader in the map and queued its own compute pipelines should call `bevy_core_pipeline::mip_generation::create_downsampling_pipelines` instead. It returns the bind group layouts and pipelines for a texture format, and adds the required shader defs itself:

```rust
let pipelines = create_downsampling_pipelines(
    &render_device,
    &pipeline_cache,
    &downsample_shaders,
    TextureFormat::Rgba16Float,
    true, // array_texture
    combine_bind_groups,
)
.unwrap();
```
