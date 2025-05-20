---
title: Use Gltf material names for spawned material entities
authors: ["@rendaoer"]
pull_requests: [19287]
---

When loading a Gltf scene in Bevy, each material is spawned as an entity, storing a `GltfMaterialName` component.

These entities were previously named e.g. `Mesh.0` and `Mesh.1`. To make these entities easier to examine in inspector-style tools, they now store the same name as in `GltfMaterialName` in their `Name` component.
