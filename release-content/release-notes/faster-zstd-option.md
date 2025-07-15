---
title: Faster Zstd decompression option
authors: ["@atlv24", "@brianreavis"]
pull_requests: [19793]
---

There is now an option to use the [zstd](https://crates.io/crates/zstd) c-bindings instead of [ruzstd](https://crates.io/crates/ruzstd).
This is less safe and portable, but can be around 44% faster.

The two features that control which one is used are `zstd_rust` and `zstd_c`.
`zstd_rust` is enabled by default, but `zstd_c` takes precedence if both are enabled.

To enable it, add the feature to the `bevy` entry of your Cargo.toml:

```toml
bevy = { version = "0.17.0", features = ["zstd_c"] }
```

Note: this will still include a dependency on `ruzstd`, because mutually exclusive features are not supported by Cargo.
To remove this dependency, disable default-features, and manually enable any default features you need:

```toml
bevy = { version = "0.17.0", default-features = false, features = [
    "zstd_c",
    "bevy_render", # etc..
] }
```
