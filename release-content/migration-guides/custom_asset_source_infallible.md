---
title: Custom asset sources now require a reader.
pull_requests: [21721]
---

Previously, it was possible to create asset sources with no reader, resulting in your asset sources
silently being skipped. This is no longer possible, since `AssetSourceBuilder` must now be given a
reader to start. We also slightly changed how sources are expected to be built.

```rust
// 0.17
AssetSource::build()
    .with_reader(move || /* reader logic */)
    .with_writer(move || /* ... */)
    .with_processed_reader(move || /* ... */)
    .with_processed_writer(move || /* ... */);

// 0.18
AssetSourceBuilder::new(move || /* reader logic */)
    .with_writer(move || /* ... */)
    .with_processed_reader(move || /* ... */)
    .with_processed_writer(move || /* ... */;
```
