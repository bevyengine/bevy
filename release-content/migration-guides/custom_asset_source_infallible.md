---
title: Custom asset sources now require a reader.
pull_requests: [21721]
---

Previously, it was possible to create asset sources with no reader, resulting in your asset sources
silently being skipped. This is no longer possible, since `AssetSourceBuilder` must now be given a
reader to start. We also slightly changed how sources are expected to be built.

In previous versions, creating a custom source would look like:

```rust
AssetSource::build()
    .with_reader(move || todo!("the reader!"))
    .with_writer(move || todo!())
    .with_processed_reader(move || todo!())
    .with_processed_writer(move || todo!())
```

In Bevy 0.18, this now looks like:

```rust
// You may need to import AssetSourceBuilder.
AssetSourceBuilder::new(move || todo!("the reader!"))
    .with_writer(move || todo!())
    .with_processed_reader(move || todo!())
    .with_processed_writer(move || todo!())
```
