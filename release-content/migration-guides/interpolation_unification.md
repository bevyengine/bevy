---
title: Unified Interpolation Traits
pull_requests: [TODO]
---

+ The `bevy_color::Mix` trait became the `bevy_math::Interpolate` trait, and `mix` is now `interp`.
+ `Color` no longer supports interpolation; you should not convert both colors into the same concrete types.
+ `VectorSpace::lerp` was replaced by `Interpolate::interp`. The `interp` method is always linear for vector-spaces.
+ `Laba`, `LinearRgba`, `Oklaba`, `Srgba` and `Xyza` no longer implement `VectorSpace`. Use `to_vec4` and `from_vec4` for color math.
