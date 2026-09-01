---
title: "TextInput"
authors: ["@viridia"]
pull_requests: []
---

The `EditableText` component has been split into two components, which are now `EditableText`
and `TextInput`. The `EditableText` component, which lives in the `bevy::text` crate, holds the
state of a text input field, but no longer has any built-in observers - that is, it does not
behave like a widget (headless or otherwise), but merely a holder of state.

All of the widget-like behaviors (responding to keystrokes) have been moved to a new `TextInput`
component which lives in the `bevy::ui_widgets` crate. Not only is the arrangement more consistent
with the other widgets, but in addition this new component also has properties which are only
interesting to widgets, like a "read-only" option.
