---
title: Clicking and typing over the Bevy Remote Protocol
authors: ["@viridia"]
pull_requests: [26333, 26342]
---

The Bevy Remote Protocol has new methods for sending input to a running app: `input.click` clicks on a UI node, `input.key` presses keys, and `input.type` types text.
These are meant for tools that drive an app from outside, such as end-to-end tests:

```json
{
    "method": "input.click",
    "params": { "entity": 4294967321 }
}
```

```json
{
    "method": "input.type",
    "params": { "text": "Hello!" }
}
```

Input arrives as a window backend would report it, one event per frame, so picking, widgets and text fields respond just as they would to a real mouse and keyboard.
A click moves the cursor to the node's center, then presses and releases the button; a node hidden inside a scrolled view is scrolled into view first.
You can also click at a point in a window, choose the mouse button, and hold it down for several frames.

`input.type` types each character as a US keyboard would, holding Shift where a character needs it.
`input.key` presses a single key, by its `KeyCode`; its logical key and text come from a US layout unless you give them, and a key can be pressed and released in separate calls to hold a modifier.

Each response names the frame that reads the last event, so a client can wait for exactly that frame before checking what the input did.

The same machinery is available in Rust as `click`, `key` and `type_text` in `bevy::dev_tools::synthetic_input`, for tools that run inside the app.
