---
title: Unified Interpolation Traits
pull_requests: [TODO]
---

+ The `bevy_color::Mix` trait became the `bevy_math::Interpolate` trait, and `mix` is now `interp`.
+ `VectorSpace::lerp` was replaced by `Interpolate::interp`. The `interp` method is always linear for vector-spaces.
