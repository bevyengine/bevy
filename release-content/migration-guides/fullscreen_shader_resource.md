---
title: `FULLSCREEN_SHADER_HANDLE` replaced with `FullscreenShader`
pull_requests: [19426]
---

`FULLSCREEN_SHADER_HANDLE` and `fullscreen_shader_vertex_state` have been replaced by the
`FullscreenShader` resource. Users of either of these will need to call `FullscreenShader::shader`
or `FullscreenShader::to_vertex_state` respectively. You may need to clone `FullscreenShader` out of
the render world to store an instance that you can use later (e.g., if you are attempting to use the
fullscreen shader inside a `SpecializedRenderPipeline` implementation).

For example, if your previous code looked like this:

```rust
struct MyPipeline {
  some_bind_group: BindGroupLayout,
}

impl FromWorld for MyPipeline {
  fn from_world(render_world: &mut World) -> Self {
    let some_bind_group = /* ... RenderDevice stuff */;
    Self {
      some_bind_group,
    }
  }
}

impl SpecializedRenderPipeline for MyPipeline {
  fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
      vertex: fullscreen_shader_vertex_state(),
      // ... other stuff
    }
  }
}
```

You can migrate your code to:

```rust
struct MyPipeline {
  some_bind_group: BindGroupLayout,
  fullscreen_shader: FullscreenShader,
}

impl FromWorld for MyPipeline {
  fn from_world(render_world: &mut World) -> Self {
    let some_bind_group = /* ... RenderDevice stuff */;
    Self {
      some_bind_group,
      fullscreen_shader: render_world.resource::<FullscreenShader>().clone(),
    }
  }
}

impl SpecializedRenderPipeline for MyPipeline {
  fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
      vertex: self.fullscreen_shader.to_vertex_state(),
      // ... other stuff
    }
  }
}
```

This is just one example. Pipelines may be initialized in different ways, but the primary strategy
is clone out the `FullscreenShader` resource from the render world, and call `to_vertex_state` to
use it as the vertex shader.
