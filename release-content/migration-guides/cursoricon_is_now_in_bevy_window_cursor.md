---
title: The `CursorIcon` API has been extracted from `bevy_winit and moved into a new crate `bevy_window_cursor`
pull_requests: [20381]
---

The `CursorIcon`, `SystemCursorIcon`, `CustomCursor`, and `CustomCursorImage` types have all been moved into a new crate `bevy_window_cursor`.

The is to make cursor customization independent of `bevy_winit`, so that it can be used with any windowing system.
