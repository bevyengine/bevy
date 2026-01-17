---
title: Diagnostics overlay
authors: ["@hukasu"]
pull_requests: [22486]
---

You can now visualize values from the `DiagnosticStore` using a `DiagnosticsOverlay` window.

An overlay can be built by spawning an entity with the [`DiagnosticsOverlay`] component
passing your custom [`DiagnosticPath`] list or using one of the provided presets.

```rust
commands.spawn(DiagnosticsOverlay::new("MyDiagnostics", vec![MyDiagnostics::COUNTER.into()]));
commands.spawn(DiagnosticsOverlay::fps());
commands.spawn(DiagnosticsOverlay::mesh_and_standard_materials());
```

By defualt the overlay will display the smoothed moving average for the diagnostic, but
you can also visualize the latest value or the moving average by passing
[`DiagnosticOverlayStatistic`]

```rust
commands.spawn(DiagnosticsOverlay::new("MyDiagnostics", vec![DiagnosticOverlayItem {
    path: MyDiagnostics::COUNTER,
    statistic: DiagnosticOverlayStatistic::Value
}]));
```
