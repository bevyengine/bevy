---
title: Consistent `*Systems` naming convention for system sets
authors: ["@Jondolf"]
pull_requests: [18900]
---

Names of `SystemSet` types within Bevy and its ecosystem have historically
been very inconsistent. Examples of system set names include `AccessibilitySystem`,
`PickSet`, `StateTransitionSteps`, and `Animation`.

Naming conventions being so wildly inconsistent can make it harder for users to pick names
for their own types, to search for system sets on docs.rs, or to even discern which types
*are* system sets.

To reign in the inconsistency and help unify the ecosystem, **Bevy 0.17** has renamed most of
its own system sets to follow a consistent `*Systems` naming convention. Renamed types include:

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

The `Systems` suffix was chosen over the other popular suffix `Set`,
because `Systems` more clearly communicates that it is specifically
a collection of systems, and it has a lower risk of naming conflicts
with other set types.

It is recommended for ecosystem crates and users to follow suit and also adopt
the `*Systems` naming convention for their system sets where applicable.
