---
title: "FeathersTextInput now includes its container"
pull_requests: [25016]
---

`FeathersTextInput` now marks and includes its decorated container by default. Read its text through the `TextInputValue` component on the root entity and update it by inserting a new value. User edits emit `ValueChange<String>` events from the root entity.

The editable text entity is now an implementation detail. Use the `leading_adornments` and
`trailing_adornments` props to place icons or other controls inside the decorated container.
