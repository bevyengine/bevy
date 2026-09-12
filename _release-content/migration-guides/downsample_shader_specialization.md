---
title: "`DownsampleShaders::general` is now a `Handle<Shader>`"
pull_requests: [25744]
---

`DownsampleShaders::general` is now a single `Handle<Shader>` instead of a `HashMap<TextureFormat, Handle<Shader>>`.

Code that looked up a shader in `DownsampleShaders::general` and queued its own compute pipelines should specialize the new `DownsamplePipeline` resource instead. It adds the required shader defs itself. Build a `DownsamplePipelineKey` for each pass, then get the pipeline from `SpecializedComputePipelines<DownsamplePipeline>` and the bind group layout from the key:

```rust
let first = DownsamplePipelineKey {
    texture_format: TextureFormat::Rgba16Float,
    array_texture: true, // for cubemaps and other 2D array textures
    combine_bind_groups: can_combine_downsampling_bind_groups(&render_adapter, &render_device),
    pass: DownsamplePass::First,
};
let second = DownsamplePipelineKey { pass: DownsamplePass::Second, ..first };

let first_pipeline = specialized_pipelines.specialize(&pipeline_cache, &downsample_pipeline, first);
let first_layout = first.bind_group_layout();
```

If you do this in a `RenderStartup` system, order it after `init_gpu_resource::<DownsamplePipeline>` and `init_gpu_resource::<SpecializedComputePipelines<DownsamplePipeline>>`, since both resources are created there.

The `FIRST_PASS` and `SECOND_PASS` shader defs in `downsample.wesl` are now `SPLIT_BIND_GROUP_FIRST` and `SPLIT_BIND_GROUP_SECOND`.
