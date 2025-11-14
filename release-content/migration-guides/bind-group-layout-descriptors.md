---
title: "`RenderPipelineDescriptor` and `ComputePipelineDescriptor` now hold a `BindGroupLayoutDescriptor`"
pull_requests: [21205]
---

In previous versions of Bevy, `RenderPipelineDescriptor` and `ComputePipelineDescriptor` held a `BindGroupLayout` to describe the layout of shader bind groups, depending directly on `wgpu`'s `BindGroupLayout`.
Now, they hold a new type `BindGroupLayoutDescriptor` which holds the `BindGroupLayoutEntry`s directly. The descriptors are used to create `BindGroupLayout`s when they are first needed by a pipeline, and cached for reuse.

Concretely, this means wherever you were using a `RenderDevice` to create a `BindGroupLayout` to store in a `RenderPipelineDescriptor` or `ComputePipelineDescriptor`, you will now create a `BindGroupLayoutDescriptor`:

```rust
// 0.17
let bind_group_layout = render_device.create_bind_group_layout(
    // ...
);
commands.insert_resource(MyPipeline {
    bind_group_layout,
    /// ...
});
// ...
let bind_group = render_context.render_device().create_bind_group(
    None
    &my_pipeline.bind_group_layout,
    // ...
);

// 0.18
let bind_group_layout = BindGroupLayoutDescriptor::new(
    // ...
);
commands.insert_resource(MyPipeline {
    bind_group_layout,
    /// ...
});
// ...
let bind_group = render_context.render_device().create_bind_group(
    None
    pipeline_cache.get_bind_group_layout(&my_pipeline.bind_group),
    // ...
);
```
