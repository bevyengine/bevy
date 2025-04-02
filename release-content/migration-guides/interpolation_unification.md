---
title: Unified Interpolation Traits
pull_requests: [TODO]
---

# Math

+ `VectorSpace::lerp` was replaced by `Interpolate::interp`. The `interp` method is always linear for vector-spaces.
+ `StableInterpolate` is now `InterpolateStable`. The `interpolate_stable` method was replaced by `Interpolate::interp`.
+ `Ease` is now `InterpolateCurve` and `interpolate_curve_unbounded` is now `interp_curve`.

# Color

+ The `bevy_color::Mix` trait became the `bevy_math::Interpolate` trait, and `mix` is now `interp`.
+ `Color` now interpolates in `Oklab` by default.
+ `Laba`, `LinearRgba`, `Oklaba`, `Srgba` and `Xyza` no longer implement `VectorSpace`. Use `scaled_by` to scale colors, or `to_vec4` and `from_vec4` for general math.
+ Every color space now implements `Interpolate`, `StableInterpolate`, and `InterpolateCurve`.

# Animation

+ A new `Blend` trait has been introduced to extend `Interpolate` with additive blending.
+ The `Animatable` trait has been replaced by `Blendable`, which has a blanket implementation for `Blend` types.
+ `AnimatableProperty` can now be defined for properties that do not implement `Animatable`/`Blendable`.
+ `BasicAnimationCurveEvaluator` is now `BlendStackEvaluator`.
+ `BasicAnimationCurveEvaluatorStackElement` is now `BlendStackElement`.
+ `AnimatableCurve` is now `PropertyCurve`.
+ `AnimatableCurveEvaluator` is now `PropertyCurveEvaluator`.
+ `AnimatableKeyframeCurve` is now `KeyframeCurve`.
