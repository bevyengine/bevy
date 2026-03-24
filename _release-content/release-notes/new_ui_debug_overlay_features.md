---
title: "New UI debug overlay features"
authors: ["@ickshonpe"]
pull_requests: [21931]
---

`UiDebugOptions` now lets you toggle outlines for border, padding, content and scrollbar regions, and optionally ignore border radius to render node outlines without curved corners. It can be used both as a `Resource` (global defaults) and as a `Component` (per-node overrides).

The scroll example was updated to outline the scrollbar bounds when the `bevy_ui_debug` feature is enabled.
