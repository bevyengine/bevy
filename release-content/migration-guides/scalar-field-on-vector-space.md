---
title: "`VectorSpace` implementations"
pull_requests: [19194]
---

Previously, implementing `VectorSpace` for a type required your type to use or at least interface with `f32`. This made implementing `VectorSpace` for double-precision types (like `DVec3`) less meaningful and useful, requiring lots of casting. `VectorSpace` has a new required associated type `Scalar` that's bounded by a new trait `ScalarField`. `bevy_math` implements this trait for `f64` and `f32` out of the box, and `VectorSpace` is now implemented for `DVec[N]` types.

If you manually implemented `VectorSpace` for any type, you'll need to implement `Scalar` for it. If you were working with single-precision floating-point types and you want the exact behavior from before, set it to `f32`.
