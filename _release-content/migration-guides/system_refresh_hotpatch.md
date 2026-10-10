---
title: "`System::refresh_hotpatch()`"
pull_requests: []
---

The `System` trait now always has a `refresh_hotpatch()` required method.
Previously, this method would only exist when the `hotpatching` feature was enabled.
This meant that manual implementations of `System` that compiled without the feature
would fail to compile if it was enabled.

Manual implementation of `System` should add this method.
If the `System` wraps other systems, it should call `refresh_hotpatch()` on the wrapped systems.
Otherwise, it should simply be empty.
