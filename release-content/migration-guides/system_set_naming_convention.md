---
title: Consistent `*Systems` naming convention for system sets
pull_requests: [18900]
---

System sets in Bevy now more consistently use a `Systems` suffix.
Renamed types include:

- `AccessibilitySystem` → `AccessibilitySystems`
- `GizmoRenderSystem` → `GizmoRenderSystems`
- `PickSet` → `PickingSystems`
- `RunFixedMainLoopSystem` → `RunFixedMainLoopSystems`
- `TransformSystem` → `TransformSystems`
- `RemoteSet` → `RemoteSystems`
- `RenderSet` → `RenderSystems`
- `SpriteSystem` → `SpriteSystems`
- `StateTransitionSteps` → `StateTransitionSystems`
- `RenderUiSystem` → `RenderUiSystems`
- `UiSystem` → `UiSystems`
- `Animation` → `AnimationSystems`
- `AssetEvents` → `AssetEventSystems`
- `TrackAssets` → `AssetTrackingSystems`
- `UpdateGizmoMeshes` → `GizmoMeshSystems`
- `InputSystem` → `InputSystems`
- `InputFocusSet` → `InputFocusSystems`
- `ExtractMaterialsSet` → `MaterialExtractionSystems`
- `ExtractMeshesSet` → `MeshExtractionSystems`
- `RumbleSystem` → `RumbleSystems`
- `CameraUpdateSystem` → `CameraUpdateSystems`
- `ExtractAssetsSet` → `AssetExtractionSystems`
- `Update2dText` → `Text2dUpdateSystems`
- `TimeSystem` → `TimeSystems`
- `EventUpdates` → `EventUpdateSystems`

To improve consistency within the ecosystem, it is recommended for ecosystem crates
and users to also adopt the `*Systems` naming convention for their system sets where applicable.
