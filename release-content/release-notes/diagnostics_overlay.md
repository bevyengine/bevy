---
title: Diagnostics overlay
authors: ["@hukasu"]
pull_requests: [22486]
---

Create a diagnostics overlay that presents in a visual manner the values of stored in the
`DiagnosticStore`.

An overlay can be built by spawning an entity with the [`DiagnosticsOverlay`] component
passing your custom [`DiagnosticPath`] list or using one of the provided presets.

```rust
commands.spawn(DiagnosticsOverlay::new("MyDiagnostics", vec![MyDiagnostics::COUNTER]));
commands.spawn(DiagnosticsOverlay::fps());
commands.spawn(DiagnosticsOverlay::mesh_and_standard_materials());
```
