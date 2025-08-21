---
title: "`bevy_render` reorganization"
pull_requests: [20485, 20330, 18703, 20587, 20502, 19997, 19991, 20000, 19949, 19943, 19953, 20498, 20496, 20493, 20492, 20491, 20488, 20487, 20486, 20483, 20480, 20479, 20478, 20477, 20473, 20472, 20471, 20470, 20392, 20390, 20388, 20345, 20344, 20051, 19985, 19973, 19965, 19963, 19962, 19960, 19959, 19958, 19957, 19956, 19955, 19954, 16620, 16619, 15700, 15666, 15650]
---

You must now import `bevy_render::NormalizedRenderTargetExt` to use methods on `NormalizedRenderTarget`
`ManualTextureViews` is now in `bevy_render::texture`

Camera types such as `Camera`, `Camera3d`, `Camera2d`, `ClearColor`, `ClearColorConfig`, `Projection`, `PerspectiveProjection`, and `OrthographicProjection` have been moved to a new crate, `bevy_camera`.
Visibility types such as `Visibility`, `InheritedVisibility`, `ViewVisibility`, `VisibleEntities`, and `RenderLayers` have been moved to `bevy_camera::visibility`.
Culling primitives such as `Frustum`, `HalfSpace`, `Aabb`, and `Sphere` have been moved to `bevy_camera::primitives`.
Import them directly or from `bevy::camera` now.

Shader types such as `Shader`, `ShaderRef`, `ShaderDef`, `ShaderCache`, and `PipelineCompilationError` have been moved to a new crate, `bevy_shader`.
Import them directly or from `bevy::shader` now.

Light types such `AmbientLight`, `PointLight`, `SpotLight`, `DirectionalLight`, `EnvironmentMapLight`, `GeneratedEnvironmentMapLight`, `LightProbe`, `IrradianceVolume`, `VolumetricFog`, `FogVolume`, and `light_consts` have been moved to a new crate, `bevy_light`.
Import them directly or from `bevy::light` now.

Mesh types such as `Mesh`, `Mesh3d`, `Mesh2d`, `MorphWeights`, `MeshBuilder`, and `Meshable` have been moved to a new crate, `bevy_mesh`.
Import them directly or from `bevy::mesh` now.

Image types such as `Image`, `ImagePlugin`, `ImageFormat`, `ImageSampler`, `ImageAddressMode`, `ImageSamplerDescriptor`, `ImageCompareFunction`, and `ImageSamplerBorderColor` have been moved to a new crate, `bevy_image`.
Import them directly or from `bevy::image` now.

Ui rendering types such as `MaterialNode`, `UiMaterial`, `UiMaterialKey`, and modules `bevy_ui::render` and `bevy_ui::ui_material` have been moved to a new crate, `bevy_ui_render`.
Import them directly or from `bevy::ui_render` now.
Furthermore, `UiPlugin` no longer has any fields. To control whether or not UI is rendered, enable or disable `UiRenderPlugin`, which is included in the DefaultPlugins.

Sprite rendering types such as `Material2d`, `Material2dPlugin`, `MeshMaterial2d`, `AlphaMode2d`, `Wireframe2d`, `TileData`, `TilemapChunk`, and `TilemapChunkTileData` have been moved to a new crate, `bevy_sprite_render`.
Import them directly or from `bevy::sprite_render` now.

`RenderAssetUsages` is no longer re-exported by `bevy_render`.
Import it from `bevy_asset` or `bevy::asset` instead.
