---
title: Implementations of `Reader` now must implement `AsyncSeek`, and `AsyncSeekForward` is deleted.
pull_requests: []
---

The `Reader` trait no longer requires implementing `AsyncSeekForward` and instead requires
implementing `AsyncSeek`. Each reader will have its own unique implementation so implementing this
will be case specific. The simplest implementation is to simply reject these seeking cases like so:

```rust
impl AsyncSeek for MyReader {
    fn poll_seek(
        self: Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
        pos: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        let forward = match pos {
            SeekFrom::Current(curr) if curr >= 0 => curr as u64,
            _ => return std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid seek mode",
            ),
        };

        // ...
    }
}
```

In addition, the `AssetReader` trait now includes a `ReaderRequiredFeatures` argument which can be
used to return an error early for invalid requests. For example:

```rust
impl AssetReader for MyAssetReader {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
        required_features: ReaderRequiredFeatures,
    ) -> Result<impl Reader, AssetReaderError> {
        match required_features.seek {
            SeekKind::Forward => {}
            SeekKind::AnySeek => return Err(UnsupportedReaderFeature::AnySeek),
        }

        // ...
    }
}
```

Since we now just use the `AsyncSeek` trait, we've deleted the `AsyncSeekForward` trait. Users of
this trait can migrate by calling the `AsyncSeek::poll_seek` method with
`SeekFrom::Current(offset)`, or the `AsyncSeekExt::seek` method.
