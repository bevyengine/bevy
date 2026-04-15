---
title: Advanced AssetServer load variants are now exposed through a builder pattern.
pull_requests: [23663]
---

In previous versions of Bevy, there were many different ways to load an asset:

- `AssetServer::load`
- `AssetServer::load_acquire`
- `AssetServer::load_untyped`
- `AssetServer::load_acquire_override_with_settings`
- etc.

All these variants have been simplified to only two variants:

1. `AssetServer::load()`: This is just a convenience and just calls the load builder internally.
2. `AssetServer::load_builder()`: allows for constructing more complex loads like untyped loads,
   loads including guards, loads with settings, etc.

Every load variant above can be reimplemented using `load_builder`, and each one of these methods
has deprecation messages on them explaining their new equivalent. For example,
`load_with_settings_override` can now be replaced by:

```rust
asset_server
    .load_builder()
    .with_settings(settings)
    .override_unapproved()
    .load(path)
```
