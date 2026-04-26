---
title: "Occlusion culling is no longer experimental"
pull_requests: [22631]
---

Occlusion culling is no longer experimental, as the known issues that could
cause Bevy to cull meshes incorrectly are now fixed.
 Consequently, the
`bevy::render::experimental::occlusion_culling` module has been renamed to
simply `bevy::render::occlusion_culling`.
