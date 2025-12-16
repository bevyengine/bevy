---
title: "`bevy_gizmos` rendering split"
pull_requests: [21536]
---

The rendering backend of `bevy_gizmos` has been split off into `bevy_gizmos_render`.
If you were using `default-features = false` and `bevy_gizmos` and `bevy_render`, you may want to enable the `bevy_gizmos_render` feature now.
