---
title: Clicking over the Bevy Remote Protocol
authors: ["@viridia"]
pull_requests: [26333]
---

The Bevy Remote Protocol has a new method, `input.click`, which clicks on a UI node in a running app.
This is meant for tools that drive an app from outside, such as end-to-end tests:

```json
{
    "method": "input.click",
    "params": { "entity": 4294967321 }
}
```

The click arrives as a window backend would report it: the cursor moves to the node's center, then the button is pressed and released, each on its own frame, so picking and widgets respond just as they would to a real mouse.
A node hidden inside a scrolled view is scrolled into view first.
You can also click at a point in a window, choose the mouse button, and hold it down for several frames.

The response names the frame that reads the release, so a client can wait for exactly that frame before checking what the click did.

The same machinery is available in Rust as `bevy::dev_tools::synthetic_input::click`, for tools that run inside the app.
