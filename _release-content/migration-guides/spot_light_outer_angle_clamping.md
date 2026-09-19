---
title: "`SpotLight::outer_angle` is clamped to a narrower range"
pull_requests: []
---

`SpotLight::outer_angle` is now clamped to `SpotLight::MIN_OUTER_ANGLE`(0.001 radians)..=`SpotLight::MAX_OUTER_ANGLE` (1.5 radians, ~85.9°). Previously only the upper bound was enforced, at just below `PI / 2.0`.

Both old bounds produced a broken cone, because `tan(outer_angle)` is the scale of the perspective projection the cone is rendered with:

- An `outer_angle` of `0.0` made that scale zero, so the shadow map projection and the shadow lookup in the shader divided by zero and rendered garbage.
- An `outer_angle` at the old clamp of `PI / 2.0 - 1e-4` made it ~10000, collapsing the whole cone into the few shadow map texels around its axis, so the nearest occluder shadowed everything and the light appeared to vanish.

The clamp is also now applied consistently: cluster assignment and the shadow map frustum previously used the unclamped angle, so they disagreed with the angle the shadow map was actually rendered with.

If you set `outer_angle` between 1.5 and `PI / 2.0`, your cone will be narrower than before. Those values did not render correctly, so this is a fix rather than a restriction; if you need a wider cone, use a `PointLight`, which uses a cubemap and has no such limit.

Clamping is silent. `SpotLight::clamped_angles` returns the `(inner_angle, outer_angle)` pair actually used for rendering, so compare against it to check whether a light's configured angles are the ones being rendered.
