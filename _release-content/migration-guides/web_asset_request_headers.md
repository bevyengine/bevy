---
title: "`WebAssetPlugin` has a new field and `WebAssetReader` is now a struct"
pull_requests: []
---

Added a `request_headers` field to `WebAssetPlugin`, for a callback that returns
the headers to send with the request for a URL. If you build the plugin with
a struct literal, fill in the rest from the default:

```rust
// Before
WebAssetPlugin {
    silence_startup_warning: true,
}

// After
WebAssetPlugin {
    silence_startup_warning: true,
    ..default()
}
```

`WebAssetReader` has to hold the callback, so it was changed from an enum to a struct.
Instead of the variants use its constructors:

- `WebAssetReader::Http` is now `WebAssetReader::http()`.
- `WebAssetReader::Https` is now `WebAssetReader::https()`.
