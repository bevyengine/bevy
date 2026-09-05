---
title: Mesh Shaders.
pull_requests: [25627]
---

Mesh shaders are now integrated with Bevy's pipeline cache and are available for advanced users to take advantage of.
Mesh shaders can be used to render

- Meshlets generated using tools like [meshoptimizer](https://meshoptimizer.org/)
- Procedural grass with dynamic level-of-detail, [as seen here](https://gpuopen.com/learn/mesh_shaders/mesh_shaders-procedural_grass_rendering/)
- Voxels, like [nvidium](https://github.com/MCRcortex/nvidium)
- Particles
- and more

Mesh shaders, at a high level, replace the classic vertex shader with a compute shader.
This allows generating geometry directly on the GPU and passing those generated primitives directly to the fragment shader without using multiple pipelines or intermediary buffers (to pass data from a compute shader to a render pipeline).

A `MeshPipeline` contains:

- an optional task shader (also known as amplification shader)
- a mesh shader
- a fragment shader

The new `MeshPipelineDescriptor` can be used to define a `MeshPipeline`.
That `MeshPipeline` is then used as a `RenderPipeline`, which allows the re-use of Bevy's lower level rendering APIs such as `RenderContext::begin_tracked_render_pass` to take advantage of the new `draw_mesh_tasks` APIs.

```rust
let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
    label: Some("custom_mesh_shader_pass"),
    color_attachments: &[Some(target.get_color_attachment())],
    depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
    ..default()
});

pass.set_render_pipeline(mesh_pipeline);
pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);

// draw_mesh_tasks dispatches the task shader if there is one,
// or dispatches the mesh shader if there is no task shader.
pass.draw_mesh_tasks(1, 1, 1);
```

It is notable that mesh shaders are an advanced graphics approach with platform-specific performance considerations, and that this is the initial base support for the feature.
Higher level user APIs, and easy integration with Bevy's `StandardMaterial`, are left to future work.

Mesh shaders are not supported on web platforms.

Check out the new `mesh_shader_intro` example for more usage examples.
