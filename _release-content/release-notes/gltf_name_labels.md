---
title: Assets in glTF files can now be referenced by name.
authors: ["@andriyDev"]
pull_requests: []
---

In previous versions of Bevy, subassets from a glTF file needed to be loaded by the **index** in the
glTF file. This would look like:

```rust
let mesh1 = asset_server.load::<Mesh>("my.gltf#Mesh5/Primitive2");
// OR
let mesh1 = asset_server.load::<Mesh>(GltfAssetLabel::Primitive {
    mesh: 5,
    primitive: 2,
}.from_asset("my.gltf"));
```

This is unfortunate though because **the index is arbitrary**. Tools like Blender could rearrange
the order of subassets, making your asset paths no longer point to what it did before. glTF has a
solution for this: names! But up until now, the only way to use these names was by loading the root
asset as a `Gltf`, then accessing it from `Assets<Gltf>`, then lookup the correct handle (for
meshes this would mean accessing `Gltf::named_meshes`, then looking up the `GltfMesh`, and then
finding the correct primitive in that).

Now, glTF files can be loaded using names! So the above example could become:

```rust
let mesh1 = asset_server.load::<Mesh>("my.gltf#Primitive:FireHydrant/2");
// OR
let mesh1 = asset_server.load::<Mesh>(GltfNamedAssetLabel::Primitive {
    mesh: "FireHydrant",
    primitive: 2,
}.from_asset("my.gltf"));
```

This can be enabled through:

1. Enabling the new `gltf_named_subassets_default` feature on `bevy_gltf`. This is a temporary
   feature, and will be removed in Bevy 0.20 when this behavior becomes the default.
2. Setting `GltfPlugin::label_mode` to `GltfLabelMode::Names`.
3. Setting `GltfLoaderSettings::label_mode` to `GltfLabelMode::Names`.

Enabling this feature will prevent loading unnamed subassets. Only named subassets can be loaded by
name. Unnamed subassets can still be access through the `Gltf` asset if necessary.

In addition, enabling this may prevent loading of some glTF files if there are non-unique names (per
subasset type). Some tools (e.g., Blender) enforce unique names, so this should not break most glTF
files.
