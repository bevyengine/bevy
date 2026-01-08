---
title: "AssetPath::resolve and resolve_embed renamed, new variants added"
pull_requests: [22416]
---

## AssetPath::resolve and resolve_embed renamed, new variants added

`AssetPath::resolve` and `AssetPath::resolve_embed` now accept `&AssetPath` instead of `&str`. The string-based variants have been renamed to `resolve_str` and `resolve_embed_str`.

## Breaking Changes

**Method renames:**
- `AssetPath::resolve(&str)` → `AssetPath::resolve_str(&str)`
- `AssetPath::resolve_embed(&str)` → `AssetPath::resolve_embed_str(&str)`

**New methods:**
- `AssetPath::resolve(&AssetPath)` - allocation-free variant
- `AssetPath::resolve_embed(&AssetPath)` - allocation-free variant

## How do I migrate?

**Before:**
```rust
let base = AssetPath::parse("a/b.gltf");
let rel = AssetPath::parse("c.bin");
let resolved = base.resolve_str(&rel.to_string()).unwrap();
```

**After:**
```rust
let base = AssetPath::parse("a/b.gltf");
let rel = AssetPath::parse("c.bin");
let resolved = base.resolve(&rel);
```

### If you're using string inputs

**Before:**
```rust
let base = AssetPath::parse("a/b.gltf");
let resolved = base.resolve("c.bin").unwrap();
```

**After:**
```rust
let base = AssetPath::parse("a/b.gltf");
let resolved = base.resolve_str("c.bin").unwrap();
```

Both variants have identical semantics:
- Label-only paths (e.g. `#label`) replace the base label
- Paths starting with `/` are rooted at the asset source root
- Explicit sources (e.g. `name://...`) replace the base source
- Path segments are normalized (`.` / `..` removal)

`resolve_embed` and `resolve_embed_str` additionally use RFC 1808-style "file portion removal" before concatenation (unless the base ends with `/`).

**Note:** Semantics are unchanged - only method names changed. The new `&AssetPath` variants avoid string allocation and parsing overhead.