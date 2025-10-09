---
title: "`RadioButton`, `RadioGroup` widget minor improvements"
authors: ["@PPakalns"]
pull_requests: [21294]
---

`RadioButton` and `RadioGroup` usage remains fully backward compatible.

Improvements:

- Event propagation from user interactions will now be canceled even if
  widgets are disabled. Previously, some relevant event propagation
  was not properly canceled.
- `RadioButton` now emits a `ValueChange<bool>` entity event when checked,
  even when checked via a `RadioGroup`. Consistent with other `Checkable` widgets.
  As a `RadioButton` cannot be unchecked through direct user interaction with this widget,
  a `ValueChange` event with value `false` can not be triggered for `RadioButton`.
- If a `RadioButton` is focusable, a value change event can be triggered
  using the **Space** or **Enter** keys when focused.
- `RadioGroup` is now optional and can be replaced with a custom implementation.
