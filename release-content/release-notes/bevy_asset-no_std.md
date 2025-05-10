---
title: Support for `no_std` in `bevy_asset`
authors: ["@bushrat011899"]
pull_requests: [19070]
---

The `bevy_asset` crate now has initial `no_std` support.

As a part of the 0.16 release cycle, there was a substantial push towards `no_std` support in Bevy, allowing its use on a wider selection of platforms.
A particularly notable crate that missed out on that initial push was `bevy_asset`.
Unlike many other Bevy crates, `bevy_asset` has significant interaction with the standard library for filesystem integration.
This posed a significant design challenge, as the Rust standard library is _fantastic_, and we want users who have access to it to be able to benefit from all the functionality it provides.

After careful consideration, we've isolated a subset of `bevy_asset` which should allow for critical functionality across all platforms without any compromises for more typical use cases.
Simply disable default features to deactivate the new `std` feature:

```toml
bevy_asset = { version = "0.17", default-features = false }
```

This release allows for using the `Asset` trait, `Handle` and `AssetPath` types, and the `Assets` resources, among other supporting items.
In the future, we hope to extend this list to include the various loading and saving traits, and the `AssetServer`.
