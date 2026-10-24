---
title: ProcessContext::load_source_asset returns typed output.
pull_requests: [25967]
---

`ProcessContext::load_source_asset` now returns a `LoadedAsset<L::Asset>`
instead of `ErasedLoadedAsset`. To maintain behavior, update your code like
this:

```rust
// Before:

let erased_loaded_asset: ErasedLoadedAsset =
  context.load_source_asset::<GltfLoader>(
    &GltfSettings::default()
  ).unwrap();

// After:

let erased_loaded_asset: ErasedLoadedAsset = ErasedLoadedAsset::from(
  context.load_source_asset::<GltfLoader>(
    &GltfSettings::default()
  ).unwrap()
);
```

Note: if you're passing this directly into `TransformedAsset::from_loaded`, you
shouldn't need this migration, since `TransformedAsset::from_loaded` now also
takes `LoadedAsset`.
