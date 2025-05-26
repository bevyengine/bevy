---
title: Use Gltf material names for spawned primitive entities
authors: ["@rendaoer"]
pull_requests: [19287]
---

When loading a Gltf scene in Bevy, each mesh primitive will generate an entity and store a `GltfMaterialName` component and `Name` component.

The `Name` components were previously stored as mesh name plus primitive index - for example, `MeshName.0` and `MeshName.1`. To make it easier to view these entities in Inspector-style tools, they are now stored as mesh name plus material name - for example, `MeshName.Material1Name` and `MeshName.Material2Name`.

If you were relying on the previous value of the `Name` component on meshes, use the new `GltfMeshName` component instead.
