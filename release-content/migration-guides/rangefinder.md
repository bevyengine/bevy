---
title: "`ViewRangefinder3d::from_world_from_view` now takes `Affine3A` instead of `Mat4`"
pull_requests: [20707]
---

`ViewRangefinder3d::from_world_from_view` now takes `Affine3A` instead of `Mat4`. If you were supplying a `GlobalTransform::to_matrix()`, simply use `GlobalTransform::affine()` now. Performance will be better.
