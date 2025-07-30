---
title: "`Single` and `Populated` now fail instead of skipping"
pull_requests: [19489, 18765]
---

`Single<D, F>` and `Populated<D, F>` now cause systems to fail instead of skip.

The introduction of `If` makes it possible to skip systems for any invalid parameter, such as `If<Res<R>>`.
The change to the behavior of `Single` and `Populated` keeps them consistent with other parameters,
and makes it possible to use them as assertions instead of only as run conditions.

Replace `Single<D, F>` with `If<Single<D, F>>`
and `Populated<D, F>` with `If<Populated<D, F>>`
to restore the old behavior.
