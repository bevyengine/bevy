---
title: Unified Interpolation Traits
pull_requests: [TODO]
---

# Math

+ The `bevy_color::Mix` trait became the `bevy_math::Interpolate` trait, and `mix` is now `interp`.
+ `Color` no longer supports interpolation. All other color types now implement `Interpolate`. You should convert to a specific color space to do interpolation.
+ `Laba`, `LinearRgba`, `Oklaba`, `Srgba` and `Xyza` no longer implement `VectorSpace`. Use `to_vec4` and `from_vec4` for color math.
+ `VectorSpace::lerp` is now `Interpolate::interp`. The `interp` method is always linear for vector-spaces.
+ `StableInterpolate::interpolate_stable` is now `Interpolate::interp`. The semantics are unchanged.

# Animation

+ A new `Blend` trait has been introduced to extend `Interpolate` with additive blending.
+ The `Animatable` trait has been replaced by `Blendable`, which has a blanket implementation for `Blend` types.
+ `BasicAnimationCurveEvaluator` is now `BlendStackEvaluator`.
+ `BasicAnimationCurveEvaluatorStackElement` is now `BlendStackElement`.
+ `AnimatableCurve` is now `PropertyCurve`.
+ `AnimatableCurveEvaluator` is now `PropertyCurveEvaluator`.
+ `AnimatableKeyframeCurve` is now `KeyframeCurve`.
+ `Ease` is now `InterpolateCurve` and `interpolate_curve_unbounded` is now `interp_curve`.
