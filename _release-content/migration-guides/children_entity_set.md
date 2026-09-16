---
title: Children no longer derefs to a Entity slice
pull_requests: [25811]
---

`Children` now wraps a `EntityIndexSet` for better performance and safety. Due to this it no longer `Deref`s to a `[Entity]`. Most operations will continue working as before, but some might have a slightly different signature. Most notably `get`/`get_mut` must now be replaced with `get_index`/`get_index_mut`.
