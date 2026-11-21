---
title: "`AssetServer::get_path_asset_loader` renamed to `get_asset_loader_with_path`"
pull_requests: [26556]
---

The `AssetServer::get_path_asset_loader` method has been renamed to
`AssetServer::get_asset_loader_with_path` for consistency with the other
`AssetServer::get_asset_loader_with_*` methods.

The old method still exists but is deprecated and simply forwards to the new one.
Replace all references:

```rust
// Before
let loader = asset_server.get_path_asset_loader(path).await?;

// After
let loader = asset_server.get_asset_loader_with_path(path).await?;
```
