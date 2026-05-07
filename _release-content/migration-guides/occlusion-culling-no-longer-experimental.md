---
title: "Occlusion culling is no longer experimental"
pull_requests: [22631]
---

Occlusion culling is no longer experimental, as all known issues that caused Bevy to cull meshes incorrectly are fixed.
 Consequently, the
`bevy::render::experimental::occlusion_culling` module has been renamed to
simply `bevy::render::occlusion_culling`.
