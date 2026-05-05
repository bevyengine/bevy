---
title: "User settings"
authors: ["@viridia", "@mpowell90"]
pull_requests: [23034, 23719, 23812]
---

The new `bevy_settings` crate provides a framework for user settings and persistent preferences.
This can include things like:

- Music and sound volume controls
- Graphics options
- Window position and size
- Whether or not to show the game tutorial
- "Don't show this dialog again"

In general, a user preference is any persistent property that is set by user action (either
explicitly or implicitly), and whose lifetime isn't limited to a single saved game file.

Preferences are defined using `bevy_reflect` annotations, and are automatically inserted as
resources when the settings framework starts up.

See the `examples/app/persisting_preferences` for a simple example of how to use the framework.

A special thanks to Andhrimnir (@tecbeast42) for giving Bevy ownership of the `bevy_settings` crate name.
