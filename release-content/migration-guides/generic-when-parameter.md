---
title: Generic `When` Parameter
pull_requests: [TODO]
---

`Single<D, F>` and `Populated<D, F>` now cause systems to fail instead of skip.

The introduction of `When` makes it possible to skip systems for any invalid parameter, such as `When<Res<R>>`.
The change to the behavior of `Single` and `Populated` keeps them consistent with other parameters,
and makes it possible to use them as assertions instead of only as run conditions.

Replace `Single<D, F>` with `When<Single<D, F>>`
and `Populated<D, F>` with `When<Populated<D, F>>`
to restore the old behavior.
