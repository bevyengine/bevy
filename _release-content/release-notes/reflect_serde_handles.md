---
title: Serializing and deserializing asset handles
authors: ["@andriyDev"]
pull_requests: [23329]
---

Asset handles can now be round-tripped successfully during serialization and deserialization.
This is particularly important for world assets — the serialization format written through `DynamicWorld::serialize`, previously called scenes.

This wasn't a matter of just slapping on some derives, because handles aren't raw data: they're a pointer to the actual loaded asset.
As a result, there was no clear way to either persist or reconstruct one.
The new `HandleSerializeProcessor` and `HandleDeserializeProcessor` solve this by storing a handle's identifying information (its asset path) on serialization, then reloading the asset from that path on deserialization. Pass them to `TypedReflectSerializer::with_processor` and `TypedReflectDeserializer::with_processor` if you need the same behavior in your own serialization pipelines.

## Caveat

For this to work, your assets need to be correctly reflected. If your asset looks like:

```rust
#[derive(Asset, TypePath)]
struct MyAsset {
    ...
}
```

Change it to:

```rust
#[derive(Asset, Reflect)]
#[reflect(Asset)]
struct MyAsset {
    ...
}
```
