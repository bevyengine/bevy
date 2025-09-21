---
title: "`Anchor` is now a required component on `Sprite`"
pull_requests: [18393]
---

The `anchor` field has been removed from `Sprite`. Instead the `Anchor` component is now a required component on `Sprite`.

The anchor variants have been moved to associated constants, following the table below:

| 0.16                  | 0.17                  |
| --------------------- | --------------------- |
| Anchor::Center        | Anchor::Center        |
| Anchor::BottomLeft    | Anchor::BOTTOM_LEFT   |
| Anchor::BottomCenter  | Anchor::BOTTOM_CENTER |
| Anchor::BottomRight   | Anchor::BOTTOM_RIGHT  |
| Anchor::CenterLeft    | Anchor::CENTER_LEFT   |
| Anchor::CenterRight   | Anchor::CENTER_RIGHT  |
| Anchor::TopLeft       | Anchor::TOP_LEFT      |
| Anchor::TopCenter     | Anchor::TOP_CENTER    |
| Anchor::TopRight      | Anchor::TOP_RIGHT     |
| Anchor::Custom(value) | Anchor(value)         |
