---
title: "Val calc"
authors: ["@ickshonpe"]
pull_requests: []
---

`Val` now has a `Calc` variant that calculates a length with a function (similar to the CSS `calc` function).

`Calc`s are constructed using the `calc` method:

```rust
Val::calc(|ctx| (ctx.percent(50.) - 10.).max(ctx.vh(25.)))
```

The function is given a context that converts lengths in each unit to logical pixels, and returns a length in logical pixels.
It can't capture its environment, so `Val` is still `Copy` and the same size as before.

Grid tracks can be sized with a function too:

```rust
GridTrack::calc(|ctx| ctx.vw(25.).clamp(200., 400.))
```
