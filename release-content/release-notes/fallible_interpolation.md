---
title: Fallible Interpolation
authors: ["@viridia"]
pull_requests: [21633]
---

## Fallible Interpolation

The `StableInterpolate` trait is great, but sadly there's one important type that it doesn't work
with: The `Val` type from `bevy_ui`. The reason is that `Val` is an enum, representing different
length units such as pixels and percentages, and it's not generally possible or even meaningful to
try and interpolate between different units.

However, the use cases for wanting to animate `Val` don't require mixing units: often we just want
to slide or stretch the length of a widget such as a toggle switch. We can do this so long as we
check at runtime that both interpolation control points are in the same units.

The new `TryStableInterpolate` trait introduces the idea of interpolation that can fail, by returning
a `Result`. Note that "failure" in this case is not necessarily bad: it just means that the
animation player will need to modify the parameter in some other way, such as "snapping" or
"jumping" to the new keyframe without smoothly interpolating. This lets us create complex animations
that incorporate both kinds of parameters: ones that interpolate, and ones that don't.

There's a blanket implementation of `TryStableInterpolate` for all types that impl
`StableInterpolate`, and these can never fail. There are additional impls for `Color` and `Val`
which can fail if the control points are not in the same units / color space.
