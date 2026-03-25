---
title: Morph targets are now stored in meshes.
pull_requests: [23023, 23485]
---

Previously, morph targets were stored as a `Handle<Image>` in a `Mesh`. Now, morph targets are
stored inside the `Mesh` itself.

As a consequence, `Gltf` assets no longer provide a `GltfAssetLabel::MorphTarget` subasset. This
subasset can be replaced with the corresponding `GltfAssetLabel::Primitive` to look up the correct
`Mesh`, followed by `Mesh::get_morph_targets`.
