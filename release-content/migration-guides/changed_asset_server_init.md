---
title: Changes to `AssetServer` and `AssetProcessor` creation.
pull_requests: [21763]
---

Previously `AssetServer`s `new` methods would take `AssetSources`. Now, these methods take
`Arc<AssetSources>`. So if you previously had:

```rust
AssetServer::new(
    sources,
    mode,
    watching_for_changes,
    unapproved_path_mode,
)

// OR:
AssetServer::new_with_meta_check(
    sources,
    mode,
    meta_check,
    watching_for_changes,
    unapproved_path_mode,
)
```

Now you need to do:

```rust
AssetServer::new(
    Arc::new(sources),
    mode,
    watching_for_changes,
    unapproved_path_mode,
)

// OR:
AssetServer::new_with_meta_check(
    Arc::new(sources),
    mode,
    meta_check,
    watching_for_changes,
    unapproved_path_mode,
)
```

`AssetProcessor::new` has also changed. It now returns to you the `Arc<AssetSources>` which can (and
should) be shared with the `AssetServer`. So if you previously had:

```rust
let processor = AssetProcessor::new(sources);
```

Now you need:

```rust
let (processor, sources_arc) = AssetProcessor::new(
    sources,
    // A bool whether the returned sources should listen for changes as asset processing completes.
    false,
);
```
