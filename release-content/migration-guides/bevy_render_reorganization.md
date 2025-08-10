---
title: `bevy_render` reorganization
pull_requests: [19997, 19991, 20000, 19949, 19943, 19953]
---

You must now import `bevy_render::NormalizedRenderTargetExt` to use methods on NormalizedRenderTarget
`ManualTextureViews` is now in `bevy_render::texture`

Camera and visibility types have been moved to a new crate, `bevy_camera`, but continue to be re-exported by `bevy_render` for now.
Import them directly or from `bevy::camera` now, as the re-exports will be removed.

Shader types have been moved to a new crate, `bevy_shader`, but continue to be re-exported by `bevy_render` for now.
Import them directly or from `bevy::shader` now, as the re-exports will be removed.

Light types have been moved to a new crate, `bevy_light`, but continue to be re-exported by `bevy_render` for now.
Import them directly or from `bevy::light` now, as the re-exports will be removed.

Mesh types have been moved to a new crate, `bevy_mesh`, but continue to be re-exported by `bevy_render` for now.
Import them directly or from `bevy::mesh` now, as the re-exports will be removed.

Image types have been moved to a new crate, `bevy_image`, but continue to be re-exported by `bevy_render` for now.
Import them directly or from `bevy::image` now, as the re-exports will be removed.

RenderAssetUsages is no longer re-exported by `bevy_render`. Import it from `bevy_asset` instead.
