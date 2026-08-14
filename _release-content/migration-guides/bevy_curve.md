---
title: "split out `bevy_curve` from `bevy_math`"
pull_requests: [25380]
---

`bevy_curve` is a new crate providing the data structures and traits to create
and sample mathematical curves. This functionality is nothing new and was
originally included in `bevy_math`. In an effort to clean up `bevy_math` it was
split out in its own crate.

The new crate means two things:

- the `curve` feature in `bevy_math` (which was default-enabled) is gone now
- imports have to be adjusted
  - from `bevy_math::curve::*` -> `bevy_curve::*`
  - from `bevy_math::cubic_splines::*` -> `bevy_curve::cubic_splines::*`
  - `bevy_math::Curve` was a top-level export, which is gone now and also just
    `bevy_curve::Curve` now.

The crate is in the `bevy` default feature set, so if you use this, nothing
should change. All the imports should be exported via `bevy::prelude::*` in
that case.

If you disable the default features, you have to enable `bevy/bevy_curve` now
instead of `bevy_math/curve`.
