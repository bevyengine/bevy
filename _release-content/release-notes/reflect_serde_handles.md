---
title: Serializing and deserializing asset handles for reflection.
authors: ["@andriyDev"]
pull_requests: [23329]
---

Asset handles are not just data: they are a reference to an asset that also keeps that asset alive.
This poses a challenge for deserializing handles: there's no asset to keep alive when deserializing!
In particular, our world serialization format (writable through `DynamicWorld::serialize`, previously called "scenes") has been unfortunately
restricted by the fact that handles could not be serialized or deserialized. A lot of things you
want to put into a world asset, like 3D models or even other scenes, need to reference asset handles for
their data.

To resolve this, we've introduced `HandleSerializeProcessor` and `HandleDeserializeProcessor` to
be used with `TypedReflectSerializer::with_processor` and `TypedReflectDeserializer::with_processor`
respectively. These allow the reflection (de)serialization to store and load handles! Serializing a
handle will store its "identifying" information (e.g., asset path), and deserializing the handle
will load the asset path to produce the handle.

In addition, this now happens automatically for world asset loading and saving!

While it isn't practical for us to directly support `serde::Serialize` and `serde::Deserialize`
(since these don't allow passing the `AssetServer` needed to execute loads), reflection allows us to
bypass these concerns and provide a reasonable API, and we expect most users to be using reflection
when wanting to serialize/deserialize handles anyway.

## Caveat

The important point to make this work is making sure your assets are correctly reflected. For
example, if your asset looks like:

```rust
#[derive(Asset, TypePath)]
struct MyAsset {
    ...
}
```

Change this to:

```rust
#[derive(Asset, Reflect)]
#[reflect(Asset)]
struct MyAsset {
    ...
}
```

For generic assets, you will also need to explicitly register each variant using
`app.register_type::<A>()` (just like any generic type).
