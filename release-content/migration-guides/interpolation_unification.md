---
title: Unified Interpolation Traits
pull_requests: [TODO]
---

+ The `bevy_color::Mix` trait became the `bevy_math::Interpolate` trait, and `mix` is now `interp`.
+ `Color` no longer supports interpolation; convert both colors into the same concrete type then call `interp`.
+ `Laba`, `LinearRgba`, `Oklaba`, `Srgba` and `Xyza` no longer implement `VectorSpace`. Use `to_vec4` and `from_vec4` for color math.
+ `VectorSpace::lerp` was replaced by `Interpolate::interp`. The `interp` method is always linear for vector-spaces.
+ `StableInterpolate` is now a sub-trait of `Interpolate`.
