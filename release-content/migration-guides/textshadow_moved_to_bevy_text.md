---
title: `TextShadow` has been moved into `bevy_text`
pull_requests: [19532]
---

The `TextShadow` component has been moved from `bevy_ui::ui_node` to `bevy_text::text`. 

This is to allow other text rendering implementations to support `TextShadow` like `Text2d` without depending `bevy_ui` to import the component.



