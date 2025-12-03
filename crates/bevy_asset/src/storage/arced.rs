use alloc::sync::Arc;

use super::{AssetSnapshotStrategy, AssetStorageStrategy, AssetWriteStrategy};

macro_rules! panic_asset_erased {
    () => {
        panic!(
            "This {} asset has been erased",
            ::core::any::type_name::<Self>()
        )
    };
}

/// This storage strategy wraps assets in an [`Arc`] so that they can be shared between threads.
/// This provides great read performance, at the expense of needing to clone the asset when the
/// asset is mutated (if there are outstanding references).
///
/// ## Ideal Usage
/// Best for heavy asset types (like images and meshes) that don't experience repeated updates.
pub struct ArcedAssetStorage;

impl<A: Send + Sync> AssetStorageStrategy<A> for ArcedAssetStorage {
    type AssetStorage = Option<Arc<A>>;

    type AssetRef<'a>
        = &'a A
    where
        A: 'a;

    #[inline]
    fn new(asset: A) -> Self::AssetStorage {
        Some(Arc::new(asset))
    }
    #[inline]
    fn get_ref<'a>(stored_asset: &'a Self::AssetStorage) -> &'a A {
        stored_asset
            .as_ref()
            .unwrap_or_else(|| panic_asset_erased!())
            .as_ref()
    }
    #[inline]
    fn into_inner(stored_asset: Self::AssetStorage) -> Option<A> {
        Arc::into_inner(stored_asset?)
    }
}

impl<A: Send + Sync + Clone> AssetWriteStrategy<A> for ArcedAssetStorage {
    type AssetMut<'a>
        = &'a mut A
    where
        A: 'a;

    #[inline]
    fn get_mut<'a>(stored_asset: &'a mut Self::AssetStorage) -> &'a mut A {
        Arc::make_mut(
            stored_asset
                .as_mut()
                .unwrap_or_else(|| panic_asset_erased!()),
        )
    }
}

impl<A: Send + Sync> AssetSnapshotStrategy<A> for ArcedAssetStorage {
    type AssetSnapshot = Arc<A>;

    #[inline]
    fn get_snapshot(stored_asset: &mut Self::AssetStorage) -> Arc<A> {
        stored_asset
            .as_ref()
            .unwrap_or_else(|| panic_asset_erased!())
            .clone()
    }
    #[inline]
    fn get_snapshot_erased(stored_asset: &mut Self::AssetStorage) -> Self::AssetSnapshot {
        stored_asset.take().unwrap_or_else(|| panic_asset_erased!())
    }
}
