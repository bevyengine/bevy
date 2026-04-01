---
title: "`Ref` now directly implements `Clone` and `Copy`"
pull_requests: [23549]
---

`Ref` now implements `Clone` and `Copy`, which means calling `ref.clone()` now
returns another `Ref<T>` rather than a cloned inner `T`. To continue cloning the
inner `T`, use `ref.as_ref().clone()`, `ref.deref().clone()`, or `ref.into_inner().clone()`.
