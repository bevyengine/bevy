---
title: Ptr::as_ptr now returns a *const u8.
pull_requests: [25745]
---

In previous versions, `Ptr::as_ptr` returned a `*mut u8`. However, `Ptr` is intended to be like an
immutable borrow. To make these semantics clearer, `Ptr::as_ptr` now returns `*const u8`.

To maintain the previous behavior, simply cast the `*const u8` to `*mut u8`. For example:
`my_ptr.as_ptr().cast_mut()`.
