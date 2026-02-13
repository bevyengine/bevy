---
title: "`experimental_ui_widgets` and `experimental_bevy_feathers` features are no longer experimental"
pull_requests: [22934]
---

The `experimental_bevy_ui_widgets` and `experimental_bevy_feathers` features have been renamed to `bevy_ui_widgets` and `bevy_feathers` respectively.

The `ui_widgets` feature has been added to the `ui` feature collection (and thus `bevy`'s default features) for ease of use.
The `bevy_feathers` feature remains off by default: it is primarily intended for use in dev tools, and should typically not be included in shipped end-user applications. As a result, it needs to be easy to enable and disable conditionally. This would be very challenging if it were a default feature or in a popular feature collection.

These crates remain immature, and subject to heavy breaking changes, even relative to Bevy's pre-1.0 standards.
However, they are useful enough to see wider adoption, and this changes substantially improves the user experience when setting up new projects
and running Bevy examples.
