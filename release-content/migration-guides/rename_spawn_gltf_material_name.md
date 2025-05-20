---
title: Use Gltf material names for spawned material entities
authors: ["@rendaoer"]
pull_requests: [19287]
---

When loading a Gltf scene in Bevy, each material will generate an entity and store a `GltfMaterialName` component.

These entities were previously named, for example, `MeshName.0` and `MeshName.1`. To make it easier to view these entities in Inspector-style tools, they are now stored in the `Name` component with the MeshName plus the MaterialName, for example, `MeshName.Material1Name` and `MeshName.Material2Name`. If the Mesh has no material, it will continue to keep `MeshName`
