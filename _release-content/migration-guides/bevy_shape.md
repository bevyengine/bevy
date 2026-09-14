---
title: "split out `bevy_shape` from `bevy_math`"
pull_requests: [25302]
---

`bevy_shape` is a new crate centered around the geometric primitives provided
by bevy. These primtiives and related traits have been split out from
`bevy_math` and are now available from different import paths than they used to
be.

Notably:

- all the `bevy_math::primitives::*` are now exposed either on the top level
  via `bevy_shape::*` or in the `bevy_shape::prelude::*`
- the following traits have also moved from `bevy_math` into `bevy_shape`
  - `Primitive2d` & `Primitive3d`
  - `Bounded2d` & `Bounded3d`
  - `BoundingVolume` & `IntersectsVolume`
  - `ToRing`
  - `Inset`
  - `ShapeSample`

If you use the `bevy::prelude::*`, there should be nothing you have to change
as all of this is still included in the general prelude. Otherwise you might
need to include a dependency on `bevy_shape` now and import your desired
structures from there.

Note that `Bounded2d`, `Bounded3d`, `BoundingVolume` & `IntersectsVolume` are
also included in the prelude now, which wasn't the case before. You can check,
if the imports of these in your code base are still necessary.
