---
title: "Some bevy_camera primitives moved to bevy_math"
pull_requests: [22684]
---

`bevy_camera::primitives::HalfSpace` has moved to `bevy_math::primitives::HalfSpace`.
Some parts of `bevy_camera::primitives::Frustum` have moved to `bevy_math::primitives::ViewFrustum`.

`bevy_camera` has some rendering primitives that can be extracted to be more generally useful.
To expose them for others to use, some of these primitives and/or functionality have moved to `bevy_math`.

```rust
// 0.18
use bevy_camera::primitives::{Frustum, HalfSpace}
let half_spaces: [HalfSpace; 6] = ...;
let frustum_one: Frustum = Frustum {
  half_spaces
};
let frustum_two: Frustum = Frustum::from_clip_from_world(...);

// 0.19
use bevy_math::primitives::{HalfSpace, ViewFrustum}
use bevy_camera::primitives::Frustum
let half_spaces: [HalfSpace; 6] = ...;
let frustum_one: Frustum = Frustum(
  ViewFrustum {
    half_spaces
});
let frustum_two: Frustum = Frustum(ViewFrustum::from_clip_from_world(...));
```
