---
title: "`PartialReflect::to_dynamic` now returns a `Result`"
pull_requests: [13723]
---

The method `PartialReflect::to_dynamic` now returns a `Result` with a `ReflectCloneError` error case rather than panicking.
Like the previous panicking case, this can only be triggered when attempting to call `to_dynamic` on opaque values.

As a result, all prior non-panicking call sites can safely be replaced with an `unwrap`.
However, related code may be defensively checking for this pattern;
you may be able to simplify your logic by handling the returned `Result` properly.
