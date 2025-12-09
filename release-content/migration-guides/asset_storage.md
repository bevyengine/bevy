---
title: "Changes to `Res<Assets<A>>` and `RenderAsset` to accommodate configurable storage"
pull_requests: [22015]
---

Bevy now allows Asset types to customize how they are stored within `Res<Assets<..>>`. This enables optimizations like using `Arc` for heavy asset types (like images) that need to be shared efficiently with the render world.

```rust
// Assets are stored directly on the stack (same behavior as Bevy < 0.18). A "snapshot" for the render world is a deep clone.
#[derive(Asset)]
#[asset_storage(StackAssetStorage)]
struct MyAsset { /* ... */ }

// Assets are stored as Arc<MyAsset>. A "snapshot" for the render world is a cheap Arc clone.
#[derive(Asset)]
#[asset_storage(ArcedAssetStorage)]
struct MyAsset { /* ... */ }

// If asset_storage is omitted, StackAssetStorage will be used by default.
#[derive(Asset)]
struct MyAsset { /* ... */ }
```

## `Res<Assets<A>>` Updates

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

### Changes to `remove()`

The remove method now returns the wrapped asset (a type defined by the asset's storage strategy). To unwrap it, use the `into_inner` method provided by the asset's storage strategy:

```diff
fn my_system(mut assets: ResMut<Assets<MyAsset>>) {
    // ...
-   let asset = assets.remove(id).unwrap();
+   let stored_asset = assets.remove(id).unwrap();
+   let asset = <MyAsset as Asset>::AssetStorage::into_inner(stored_asset).unwrap();
}
```

### Removal of `get_or_insert_with`

The `get_or_insert_with` method has been removed from `Res<Assets<A>>`. Replace it with separate calls to `get_mut` and `insert` as needed.

## `RenderAsset` Updates

Asset storage strategies now provide a "snapshot" method for a representation of the asset for the render world. For `ArcedAssetStorage`, this is an `Arc<Asset>`; other strategies use different wrapper types that dereference to the asset.

The signature of the `prepare_asset` method on `RenderAsset` has been updated to accept an asset's snapshot type:

```diff
+ use bevy::asset::AssetSnapshot;

impl RenderAsset for MyAsset {
    fn prepare_asset(
-       source_asset: Self::SourceAsset,
+       source_asset: AssetSnapshot<Self::SourceAsset>,
        asset_id: AssetId<Self::SourceAsset>,
        param: &mut SystemParamItem<Self::Param>,
        previous_asset: Option<&Self>,
-    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
+    ) -> Result<Self, PrepareAssetError<AssetSnapshot<Self::SourceAsset>>> {
        // ...
    }
}
```
