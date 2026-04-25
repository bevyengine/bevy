---
title: "`experimental_ui_widgets` feature is no longer experimental"
pull_requests: [22934]
---

The `experimental_bevy_ui_widgets` feature has been renamed to `bevy_ui_widgets`.

The `bevy_ui_widgets` feature has been added to the `ui` feature collection (and thus `bevy`'s default features) for ease of use.

This crate remains immature, and is subject to heavy breaking changes, even relative to Bevy's pre-1.0 standards.
However, it is useful enough to see wider adoption, and this changes substantially improves the user experience when setting up new projects
and running Bevy examples.
