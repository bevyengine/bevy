---
title: The `AssetReader` trait now takes a `ReaderRequiredFeatures` argument.
pull_requests: []
---

The `AssetReader::read` method now takes an additional `ReaderRequiredFeatures` argument. If
previously you had:

```rust
struct MyAssetReader;

impl AssetReader for MyAssetReader {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        todo!()
    }

    // more stuff...
}
```

Change this to:

```rust
struct MyAssetReader;

impl AssetReader for MyAssetReader {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
        _required_features: ReaderRequiredFeatures,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        todo!()
    }

    // more stuff...
}
```
