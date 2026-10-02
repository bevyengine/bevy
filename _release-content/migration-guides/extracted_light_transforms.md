---
title: Extracted light transforms
pull_requests: [25737]
---

Extracted lights no longer store a `GlobalTransform`. They store only the position
and orientation needed by the renderer, without scale or shear.

- Replace `ExtractedPointLight::transform.translation()` with `position`.
  For spot lights, replace `transform.forward()` with `direction` and
  `transform.back()` with `-direction`. The `direction` field is unused for point lights.
- Replace `ExtractedDirectionalLight::transform.back()` with `dir_to_light`.
- Replace `ExtractedRectLight::transform.translation()` with `position` and
  `transform.rotation()` with `rotation`. Its right and up axes are
  `rotation * Vec3::X` and `rotation * Vec3::Y`.

`spot_light_world_from_view` now takes `position: Vec3` and `direction: Dir3`
instead of `&GlobalTransform`. Replace `spot_light_world_from_view(&transform)`
with `spot_light_world_from_view(transform.translation(), transform.forward())`.
