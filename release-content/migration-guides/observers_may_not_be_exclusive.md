---
title: Exclusive systems may not be used as observers
pull_requests: [19033]
---

Exclusive systems may no longer be used as observers.
This was never sound, as the engine keeps references alive during observer invocation that would be invalidated by `&mut World` access, but was accidentally allowed.
Instead of `&mut World`, use either `DeferredWorld` if you do not need structural changes, or `Commands` if you do.
