---
title: "`ui::widgets::Button` and `ui::Interaction` are Deprecated"
pull_requests: [25197]
---

`ui::widgets::Button`, available via the `ui::prelude`, has been deprecated in favor of `ui_widgets::Button`.
`ui::Interaction` has been deprecated in favor of the `picking::hover::Hovered` and `ui::Pressed` components.

View the updated `button.rs` example for updated `Button` usage patterns and how to use the `Hovered` and `Pressed` components.
