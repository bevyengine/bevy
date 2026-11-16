---
title: Window `fit_canvas_to_parent` field now defaults true
pull_requests: [26526]
---

The `fit_canvas_to_parent` field on `Window`s now defaults to true. This setting adds `width: 100%` and `height: 100%` styles to the app's canvas on web platforms.

This behavior is expected by web publishing platforms like itch.io, however in some web layouts (especially with implicitly-sized parents) it can cause a feedback loop resulting in the canvas growing infinitely.

If your canvas starts misbehaving from this change, you can either fix the canvas parent's styling or turn the field back to false using `WindowPlugin`'s `primary_window` field.
