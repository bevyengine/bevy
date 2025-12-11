---
title: LoadContext::path now returns `AssetPath`.
pull_requests: [21713]
---

`LoadContext::asset_path` has been removed, and `LoadContext::path` now returns `AssetPath`. So the
migrations are:

- `load_context.asset_path()` -> `load_context.path()`
- `load_context.path()` -> `load_context.path().path()`
  - While this migration will keep your code running, seriously consider whether you need to use
    the `Path` itself. The `Path` does not support custom asset sources, so care needs to be taken
    when using it directly. Consider instead using the `AssetPath` instead, along with
    `AssetPath::resolve_embed`, to properly support custom asset sources.
