---
title: "`WindowEvent::CursorMoved` wraps `RawCursorMoved`"
pull_requests: [25890]
---

`WindowEvent::CursorMoved` now wraps `RawCursorMoved` instead of `CursorMoved`. It's the raw
event from the window backend, with only the window and the cursor position in physical pixels.

Reading the `CursorMoved` message is unchanged.

If you were writing a `WindowEvent::CursorMoved`, write a `RawCursorMoved` with the physical position:

If you were reading `WindowEvent::CursorMoved`, compute the logical position from the physical one
with the scale factor of the window, or read the `CursorMoved` message instead.
