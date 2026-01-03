---
title: Changes to the `Process` trait in `bevy_asset`.
pull_requests: [21925]
---

`ProcessContext` no longer includes `asset_bytes`. This has been replaced by `asset_reader`. To
maintain current behavior in a `Process` implementation, you can read all the bytes into a memory `Vec`.

```rust
// 0.17
let bytes = context.asset_bytes();

// 0.18
let reader = context.asset_reader();
let mut bytes = vec![];

reader
    .read_to_end(&mut bytes)
    .await
    .map_err(|err| ProcessError::AssetReaderError {
        path: context.path().clone_owned(),
        err: err.into(),
    })?;
```
