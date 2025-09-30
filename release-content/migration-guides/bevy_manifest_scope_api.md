---
title: "`BevyManifest::shared` is now a scope-like API."
pull_requests: [20630]
---

In previous versions of Bevy, `BevyManifest` returned a mapped `RwLock` guard. Now, it's a scope-like API:

```rust
// 0.16
let manifest = BevyManifest::shared();
let path = manifest.get_path("my_bevy_crate");

// 0.17
let path = BevyManifest::shared(|manifest| {
    manifest.get_path("my_bevy_crate")
});
```
