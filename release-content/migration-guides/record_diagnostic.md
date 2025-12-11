---
title: "`RenderDiagnostic::time_span` now takes `CommandEncoder`"
pull_requests: [ 21504 ]
---

`RenderDiagnostic::time_span` now takes `CommandEncoder` instead of being generic over `CommandEncoder`/`RenderPass`/`ComputePass`, as that led to missed feature checks. Use preexisting `RenderDiagnostic::pass_span` to create spans for render/compute passes.

```rust
let time_span = diagnostics.time_span(render_context.command_encoder(), "my span");
let pass_span = diagnostics.pass_span(&mut render_pass, "my span");
```
