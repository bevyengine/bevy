---
title: "`Res<Assets<A>>` now uses configurable storage"
pull_requests: [22015]
---

### Changes to `get()` and `get_mut()` Return Types

The return types for `get()` and `get_mut()` have changed:
- `get()` now returns `AssetRef<'_, A>` instead of `&A`
- `get_mut()` now returns `AssetMut<'_, A>` instead of `&mut A`

These new types automatically dereference to their previous equivalents (`AssetRef` → `&A`, `AssetMut` → `&mut A`), so most code will continue to work unchanged.

However, if you access multiple assets within the same scope, you may encounter borrow checker errors. To fix these, ensure each `AssetMut` or `AssetRef` is dropped before accessing another asset:

```rust
// Before (now causes borrow checker error)
let asset_a = assets.get_mut(asset_id_a).unwrap();
asset_a.field = true;
let asset_b = assets.get_mut(asset_id_b).unwrap();
asset_b.field = true;
```

```rust
// After (fixed)
{
    let asset_a = assets.get_mut(asset_id_a).unwrap();
    asset_a.field = true;
    // AssetMut dropped here
}
{
    let asset_b = assets.get_mut(asset_id_b).unwrap();
    asset_b.field = true;
    // AssetMut dropped here
}
```

### Removal of `get_or_insert_with`

The `get_or_insert_with` method has been removed from `Res<Assets<A>>`. Replace it with separate calls to `get_mut` and `insert` as needed.
