---
title: "AssetPath::resolve` and `resolve_embed` now take `&AssetPath"
pull_requests: [22416]
---

# `AssetPath::resolve` and `resolve_embed` now take `&AssetPath`

`AssetPath::resolve` and `AssetPath::resolve_embed` no longer accept `&str`. They now take `&AssetPath`. The string-based variants have been renamed to `resolve_str` and `resolve_embed_str`.

## What changed?

- `AssetPath::resolve` now takes `&AssetPath` instead of `&str`
- `AssetPath::resolve_embed` now takes `&AssetPath` instead of `&str`
- String-based variants are now `resolve_str` and `resolve_embed_str`

## Why was this changed?

This change avoids unnecessary string allocation and parsing when an `AssetPath` is already available.

## How do I migrate?

### If you already have an `AssetPath`

**Before:**
```rust
let base = AssetPath::parse("a/b.gltf");
let rel = AssetPath::parse("c.bin");
let resolved = base.resolve(&rel.to_string()).unwrap();
```

**After:**
```rust
let base = AssetPath::parse("a/b.gltf");
let rel = AssetPath::parse("c.bin");
let resolved = base.resolve(&rel);
```

### If you need to resolve from a string

**Before:**
```rust
let base = AssetPath::parse("a/b.gltf");
let rel = AssetPath::parse("c.bin");
let resolved = base.resolve(&rel.to_string()).unwrap();
```

**After:**
```rust
let base = AssetPath::parse("a/b.gltf");
let rel = AssetPath::parse("c.bin");
let resolved = base.resolve_str(&rel.to_string()).unwrap();
```

The same change applies to `resolve_embed`, which now takes `&AssetPath`. Its string-based variant is `resolve_embed_str`. Semantics are unchanged.