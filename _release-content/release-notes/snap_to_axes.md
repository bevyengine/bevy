---
title: Snap to axes
authors: ["@taishi-sama"]
pull_requests: [23674]
---

``FreeCamera`` is now able to align itself to axes on hotkey presses.

| Default Key Binding | Action                 |
|:--------------------|:-----------------------|
| `Numpad1`           | Snap to front(-Z)      |
| `LCtrl` + `Numpad1` | Snap to back(+Z)       |
| `Numpad3`           | Snap to right(+X)      |
| `LCtrl` + `Numpad3` | Snap to left(-X)       |
| `Numpad7`           | Snap to top(+Y)        |
| `LCtrl` + `Numpad7` | Snap to bottom(-Y)     |

These hotkeys can be changed by modifying ``FreeCamera`` component.
