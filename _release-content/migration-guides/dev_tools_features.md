---
title: bevy_dev_tools feature flags changed
pull_requests: [26025]
---

The `render_dev_tools` cargo feature has been removed. It's now activated automatically when the `bevy_core_pipeline` and `bevy_dev_tools` features are enabled.

`diagnostics_overlay`, `fps_overlay`, and `frame_time_graph` modules of `bevy_dev_tools` are now only available when the `bevy_ui_render` feature is enabled.
