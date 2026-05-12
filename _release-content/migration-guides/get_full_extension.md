---
title: get_full_extension now returns Option<&str>.
pull_requests: [23105]
---

Previously, `AssetPath::get_full_extension` returned `Option<String>`. Now it returns
`Option<&str>`. To keep the original behavior, change the following:

```rust
// 0.18
asset_path.get_full_extension()
```

To:

```rust
// 0.19
asset_path.get_full_extension().map(ToString::to_string)
```
