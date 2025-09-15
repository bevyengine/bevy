---
title: Move cursor-related types from `bevy_winit` to `bevy_window`
pull_requests: [20427]
---

In an effort to reduce and untangle dependencies, cursor-related types have been moved from the `bevy_winit` crate to the `bevy_window` crate.
The following types have been moved as part of this change:

- `CursorIcon` is now located at `bevy::window::CursorIcon`.
- `CustomCursor` is now located at `bevy::window::CustomCursor`.
- `CustomCursorImage` is now located at `bevy::window::CustomCursorImage`.
- `CustomCursorUrl` is now located at `bevy::window::CustomCursorUrl`.
- on the android platform, `ANDROID_APP` is now located in it's own crate and can be found at `bevy::android::ANDROID_APP`.
