---
title: get_full_extension now returns Option<&str>.
pull_requests: [14791, 15458, 15269]
---

Previously, `AssetPath::get_full_extension` returned `Option<String>`. Now it returns
`Option<&str>`. To maintain behavior, change the following:

```rust
asset_path.get_full_extension()
```

To:

```rust
asset_path.get_full_extension().map(ToString::to_string)
```
