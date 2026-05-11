---
title: Return type of `Access::archetypal` changed
pull_requests: [23384]
---

The return type of `Access::archetypal` has changed from `impl Iterator` to a new `&ComponentIdSet` type.  That type does implement `IntoIterator`, but callers may need to call the `iter()` method to get an `Iterator`.
