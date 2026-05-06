---
title: Diagnostics overlay
authors: ["@hukasu"]
pull_requests: [22486]
---

*TODO: Add a screenshot of the DiagnosticsOverlay in a real app.*

Bevy's diagnostics have always been easy to dump to the terminal, but displaying them in-game meant wiring up your own UI.
`DiagnosticsOverlayPlugin` adds a built-in overlay for this, with presets for common cases:

```rust
commands.spawn(DiagnosticsOverlay::fps());
commands.spawn(DiagnosticsOverlay::mesh_and_standard_materials());
```

You can also build a custom overlay from any [`DiagnosticPath`] list:

```rust
commands.spawn(DiagnosticsOverlay::new("MyDiagnostics", vec![MyDiagnostics::COUNTER.into()]));
```

By default the overlay shows the smoothed moving average. You can switch to the latest value or the raw moving average via [`DiagnosticsOverlayStatistic`], and configure floating-point precision with [`DiagnosticsOverlayItem::precision`]:

```rust
commands.spawn(DiagnosticsOverlay::new("MyDiagnostics", vec![DiagnosticsOverlayItem {
    path: MyDiagnostics::COUNTER,
    statistic: DiagnosticsOverlayStatistic::Value,
    precision: 4,
}]));
```
