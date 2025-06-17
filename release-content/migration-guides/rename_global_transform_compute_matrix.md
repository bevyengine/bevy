---
title: GlobalTransform::compute_matrix rename
pull_requests: [19643]
---

`GlobalTransform::compute_matrix` has been renamed to `GlobalTransform::to_matrix` because it does not compute anything, it simply moves data into a different type.
