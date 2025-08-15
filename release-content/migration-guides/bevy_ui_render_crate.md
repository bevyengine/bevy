---
title: "`bevy_ui_render` crate"
pull_requests: [18703]
---

The `render` and `ui_material` modules have been removed from `bevy_ui` and placed into a new crate `bevy_ui_render`.

As a result, `UiPlugin` no longer has any fields: add or skip adding `UiRenderPlugin` to control whether or not UI is rendered.
