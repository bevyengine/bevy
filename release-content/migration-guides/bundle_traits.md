---
title: Bundle Traits
pull_requests: [22670]
---

The function `Bundle::component_ids` is now generic over `Components: ComponentIdDictator`. 
The `ComponentIdDictator` is implemented for the old `ComponentsRegistrator` and `ComponentsQueuedRegistrator`.
Additionally, because [this rust rfc](https://github.com/rust-lang/rust/issues/130043) is not yet stable, the `use<Self>` had to be extended to `use<Self, Components>`.
This may cause some lifetime annoyances but nothing a `SmallVec` can't fix.
This was done to allow bundles to work with queued component registration.
