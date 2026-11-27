---
title: "Val calc"
authors: ["@ickshonpe"]
pull_requests: [26694]
---

`Val` now has a `Calc` variant that calculates a length with a function (similar to the CSS `calc` function).

`Calc`s are constructed using the `calc` method:

```rust
Val::calc(|ctx| (ctx.percent(50.) - 10.).max(ctx.vh(25.)))
```

The function is given a context that converts lengths in each unit to logical pixels, and returns a length in logical pixels.

`MinTrackSizingFunction` and `MaxTrackSizingFunction` can also accept `Calc`s:

```rust
GridTrack::calc(|ctx| ctx.vw(25.).clamp(200., 400.));
RepeatedGridTrack::calc(2, |ctx| ctx.percent(50.) - 8.);
GridTrack::minmax(
    MinTrackSizingFunction::Calc(Calc::new(|ctx| ctx.em(10.).max(120.))),
    MaxTrackSizingFunction::Fraction(1.),
);
GridTrack::minmax(
    MinTrackSizingFunction::Px(200.),
    MaxTrackSizingFunction::Calc(Calc::new(|ctx| ctx.percent(50.) - 16.)),
);
```
