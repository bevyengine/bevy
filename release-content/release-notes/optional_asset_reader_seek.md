---
title: The `AssetReader` trait can now (optionally) support seeking any direction.
authors: ["@andriyDev", "@cart"]
pull_requests: [22182]
---

In Bevy 0.15, we replaced the `AsyncSeek` super trait on `Reader` with `AsyncSeekForward`. This
allowed our `Reader` trait to apply to more cases (e.g., it could allow cases like an HTTP request,
which may not support seeking backwards). However, it also meant that we could no longer use seeking
fully where it was available.

To resolve this issue, we've added support for the `Reader` passed into `AssetLoader` to try casting into `SeekableReader`:

```rust
let seekable_reader = reader.seekable()?;
seekable_reader.seek(SeekFrom::Start(10)).await?;
```

`Reader` implementations that support seeking (such as the filesystem `AssetReader`) will cast successfully. If the cast fails, `AssetLoader` implementors can choose to either fail, or implement fallback behavior (such as reading all asset bytes into a `Vec`, which is seekable).
