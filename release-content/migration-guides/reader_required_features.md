---
title: The `AssetReader` trait now takes a `ReaderRequiredFeatures` argument.
pull_requests: []
---

The `AssetReader::read` method now takes an additional `ReaderRequiredFeatures` argument.

```rust
// 0.17
struct MyAssetReader;

impl AssetReader for MyAssetReader {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        // ...
    }

    // ...
}

// 0.18
struct MyAssetReader;

impl AssetReader for MyAssetReader {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
        // This is new!
        _required_features: ReaderRequiredFeatures,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        // ...
    }
}
```
