---
title: Original target of `Pointer` picking events is now stored on observers
pull_requests: [TODO]
---

The `Pointer.target` field, which tracks the original target of the pointer event before bubbling has been removed.
Instead, all observers now track this information, available via `On::original_target`.

If you were using this information via the buffered event API of picking, please migrate to observers.
If you cannot for performance reasons, please open an issue explaining your exact use case!
