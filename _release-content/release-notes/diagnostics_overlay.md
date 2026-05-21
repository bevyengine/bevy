---
title: Diagnostics overlay
authors: ["@hukasu"]
pull_requests: [22486]
---

*TODO: Add a screenshot of the DiagnosticsOverlay in a real app.*

Bevy's diagnostics have always been easy to dump to the terminal, but displaying them in-game meant wiring up your own UI.
[`DiagnosticsOverlayPlugin`] adds a built-in overlay for this with presets for common cases:

```rust
commands.spawn(DiagnosticsOverlay::fps());
commands.spawn(DiagnosticsOverlay::mesh_and_standard_materials());
```

You can also build a custom overlay from any [`DiagnosticPath`] list:

```rust
commands.spawn(DiagnosticsOverlay::new("MyDiagnostics", vec![MyDiagnostics::COUNTER.into()]));
```

By default, the overlay shows a smoothed moving average. You can show the latest value or the raw moving average instead using [`DiagnosticsOverlayItem::statistic`] and configure floating-point precision with [`DiagnosticsOverlayItem::precision`]:

```rust
commands.spawn(DiagnosticsOverlay::new("MyDiagnostics", vec![DiagnosticsOverlayItem {
    path: MyDiagnostics::COUNTER,
    statistic: DiagnosticsOverlayStatistic::Value,
    precision: 4,
}]));
```

Check out the updated [`scene_viewer` example] to see it in action!

![diagnostics overlay in the scene viewer tool example](diagnostics_overlay.jpg)

[`DiagnosticsOverlayPlugin`]: https://docs.rs/bevy/0.19.0/bevy/dev_tools/diagnostics_overlay/struct.DiagnosticsOverlayPlugin.html
[`DiagnosticsOverlayItem::statistic`]: https://docs.rs/bevy/0.19.0/bevy/dev_tools/diagnostics_overlay/struct.DiagnosticsOverlayItem.html#method.statistic
[`DiagnosticsOverlayItem::precision`]: https://docs.rs/bevy/0.19.0/bevy/dev_tools/diagnostics_overlay/struct.DiagnosticsOverlayItem.html#method.precision
