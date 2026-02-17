---
title: "`bevy_picking` feature flag removed"
pull_requests: [22933]
---

The `bevy/bevy_picking` feature flag has been removed. This previously enabled picking functionality in `bevy_input_focus`,
allowing users to select elements to focus using their mouse.

This is now exposed as part of the existing `bevy/bevy_ui_picking` feature, which is itself part of the `ui` feature collection.
In most cases, you should add the `ui` feature collection to your project if you are using `bevy_ui`.

If you want to enable `bevy_input_focus`'s picking functionality, but do *not* want to use `bevy_ui`, add a separate dependency to the same version of `bevy_input_focus` in your project and enable the optional `bevy_picking` feature there.
