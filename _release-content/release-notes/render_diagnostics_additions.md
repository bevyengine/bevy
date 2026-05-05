---
title: Render Diagnostic Additions
authors: ["@JMS55"]
pull_requests: [22326]
---

Bevy's [RenderDiagnosticPlugin](https://docs.rs/bevy/0.19.0/bevy/render/diagnostic/struct.RenderDiagnosticsPlugin.html) has new methods for uploading data from GPU buffers to bevy_diagnostic.

```rust
impl ViewNode for Foo {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        _: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let diagnostics = render_context.diagnostic_recorder();

        diagnostics.record_u32(
            render_context.command_encoder(),
            &my_buffer1.slice(..), // Buffer slice must be 4 bytes, and buffer must have BufferUsages::COPY_SRC
            "my_diagnostics/foo",
        );

        diagnostics.record_f32(
            render_context.command_encoder(),
            &my_buffer2.slice(..), // Buffer slice must be 4 bytes, and buffer must have BufferUsages::COPY_SRC
            "my_diagnostics/bar",
        );

        Ok(())
    }
}
```
