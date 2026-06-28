---
title: "`AssetId::invalid()` and `AssetId::INVALID_UUID` have been deprecated"
pull_requests: [24392]
---

`AssetId::invalid()` and `AssetId::INVALID_UUID` have been deprecated. This is
part of an effort to reduce special cases and optimize asset lookups.

If you were using `AssetId::invalid()` as a null value, the recommended solution
is to change your variable to be `Option<AssetId>` and use `None` instead of
`AssetId::invalid()`.

Before:

```rust
struct MyImageResource(AssetId<Image>);

world.insert_resource(MyImageResource(AssetId::invalid());

...

let resource = world.resource::<MyImageResource>()?;
let asset = assets.get(resource.0)?;
```

After:

```rust
struct MyImageResource(Option<AssetId<Image>>);

world.insert_resource(MyImageResource(None));

...

let resource = world.resource::<MyImageResource>()?;
let asset = assets.get(resource.0?)?;
```

In some cases it may be possible to use `AssetId::default()` instead. But note
that the default ID is *not* guaranteed to be a null value - an asset can be
registered with the default ID.
