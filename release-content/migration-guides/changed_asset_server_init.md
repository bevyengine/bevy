---
title: Changes to `AssetServer` and `AssetProcessor` creation.
pull_requests: [21763]
---

Previously `AssetServer`s `new` and `new_with_method_check` methods would take `AssetSources`. Now, these methods take
`Arc<AssetSources>`.

```rust
// 0.17
AssetServer::new(
    sources,
    mode,
    watching_for_changes,
    unapproved_path_mode,
)

// 0.18
AssetServer::new(
    // Wrap the sources in an `Arc`.
    Arc::new(sources),
    mode,
    watching_for_changes,
    unapproved_path_mode,
)
```

`AssetProcessor::new` has also changed. It now returns to you the `Arc<AssetSources>` which can (and
should) be shared with the `AssetServer`.

```rust
// 0.17
let processor = AssetProcessor::new(sources);

// 0.18
let (processor, sources_arc) = AssetProcessor::new(
    sources,
    // A bool whether the returned sources should listen for changes as asset processing completes.
    false,
);
```
