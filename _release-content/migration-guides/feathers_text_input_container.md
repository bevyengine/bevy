---
title: "FeathersTextInput now includes its container"
pull_requests: [25016]
---

`FeathersTextInput` now marks and includes its decorated container by default; it is no longer on the entity containing `EditableText`. Pass editable-entity additions via its `input` prop, leading controls via `leading_controls`, and trailing controls via `extra_controls`.
Use `FeathersTextInputBare` when embedding an input in a custom container.
Queries that combine `With<FeathersTextInput>` and `EditableText` should instead use `With<FeathersTextInputBare>`.
