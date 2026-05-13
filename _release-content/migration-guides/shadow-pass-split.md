---
title: "`shadow_pass` has been split into `per_view_shadow_pass` and `shared_shadow_pass`"
pull_requests: [23713]
---

`shadow_pass` has been split into `per_view_shadow_pass` (for rendering DirectionalLight shadow maps)
and `shared_shadow_pass` (for rendering PointLight and SpotLight shadow maps).
