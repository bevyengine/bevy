---
title: Transform and GlobalTransform::compute_matrix rename
pull_requests: [19643, 19646]
---

`GlobalTransform::compute_matrix` has been renamed to `GlobalTransform::to_matrix` because it does not compute anything, it simply moves data into a different type.
`Transform::compute_matrix` has been renamed to `Transform::to_matrix` for consistency with `GlobalTransform`.
