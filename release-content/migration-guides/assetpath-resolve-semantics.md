---
title: "`AssetPath::resolve` and `resolve_embed` now take `&AssetPath`"
pull_requests: [22416]
---

`AssetPath::resolve` and `AssetPath::resolve_embed` no longer accept `&str` and now take `&AssetPath` directly. The previous string-based APIs have been renamed to `resolve_str` and `resolve_embed_str`.

This change avoids unnecessary string allocation and parsing when an `AssetPath` is already available. To migrate, pass an `AssetPath` directly to `resolve` or `resolve_embed`; when working with strings, use the corresponding `*_str` methods instead. No behavioral or semantic changes were introduced.