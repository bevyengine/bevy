---
title: `bevy_window_cursor` crate
authors: ["@Ickshonpe"]
pull_requests: [20381]
---

In past releases the `CursorIcon` API was located in the `bevy_winit` crate. This meant that mouse cursor icon customization was tied to the `bevy_winit` backend.

In order that cursor icon customization is independent of any particular windowing system a new crate has been added, `bevy_window_cursor`. 

The `CursorIcon`, `SystemCursorIcon`, `CustomCursor`, and `CustomCursorImage` types have all moved into the `bevy_window_cursor` crate. The `CusorIcon` API is otherwise changed.
