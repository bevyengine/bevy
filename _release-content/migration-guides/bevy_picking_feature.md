---
title: "`bevy_picking` feature flag no longer includes `bevy_input_focus`"
pull_requests: [22933, 22990]
---

The `bevy/bevy_picking` feature flag no longer enables `bevy_input_focus` picking functionality.
For context, `bevy_input_focus` is inherently a `bevy_ui` related feature, allowing users to select UI elements to focus using their mouse.

Instead, this functionality is now tied to the existing `bevy/ui_picking` feature, which is itself part of the `ui` feature collection.
In most cases, you should add the `ui` feature collection to your project if you are using `bevy_ui`.

If you want to enable `bevy_input_focus`'s picking functionality, but do *not* want to use `bevy_ui`, add a separate dependency to the same version of `bevy_input_focus` in your project and enable the optional `bevy_picking` feature there.

This change means it now possible to enable `bevy_picking` without any assumptions about which backend in particular will be used.
