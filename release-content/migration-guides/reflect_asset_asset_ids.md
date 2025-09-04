---
title: "`ReflectAsset` now uses `UntypedAssetId` instead of `UntypedHandle`"
pull_requests: [19606]
---

Previously, `ReflectAsset` methods all required having `UntypedHandle`. The only way to get an
`UntypedHandle` through this API was with `ReflectAsset::add`. `ReflectAsset::ids` was not very
useful in this regard.

Now, all methods have been changed to accept `impl Into<UntypedAssetId>`, which matches our regular
`Assets<T>` API. This means you may need to change how you are calling these methods.

For example, if your code previously looked like:

```rust
let my_handle: UntypedHandle;
let my_asset = reflect_asset.get_mut(world, my_handle).unwrap();
```

You can migrate it to:

```rust
let my_handle: UntypedHandle;
let my_asset = reflect_asset.get_mut(world, &my_handle).unwrap();
```
