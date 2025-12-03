use alloc::boxed::Box;

use super::{AssetSnapshotStrategy, AssetStorageStrategy, AssetWriteStrategy};

macro_rules! panic_asset_erased {
    () => {
        panic!(
            "This {} asset has been erased",
            ::core::any::type_name::<Self>()
        )
    };
}

/// This storage strategy wraps assets in a [`Box`]. This is less preferable than [`crate::StackAssetStorage`],
/// except for cases when the stack size of the asset type is large. Boxing reduces the performance cost
/// of resizing the inner storage of [`crate::Assets<A>`] when assets are added and the capacity is exceeded.
pub struct BoxedAssetStorage;

impl<A: Send + Sync + Sized> AssetStorageStrategy<A> for BoxedAssetStorage {
    type AssetStorage = Option<Box<A>>;

    type AssetRef<'a>
        = &'a A
    where
        A: 'a;

    #[inline]
    fn new(asset: A) -> Self::AssetStorage {
        Some(Box::new(asset))
    }
    #[inline]
    fn get_ref<'a>(stored_asset: &'a Self::AssetStorage) -> &'a A {
        stored_asset
            .as_ref()
            .unwrap_or_else(|| panic_asset_erased!())
    }
    #[inline]
    fn into_inner(stored_asset: Self::AssetStorage) -> Option<A> {
        Some(*stored_asset?)
    }
}

impl<A: Send + Sync> AssetWriteStrategy<A> for BoxedAssetStorage {
    type AssetMut<'a>
        = &'a mut A
    where
        A: 'a;

    #[inline]
    fn get_mut<'a>(stored_asset: &'a mut Self::AssetStorage) -> &'a mut A {
        stored_asset
            .as_mut()
            .unwrap_or_else(|| panic_asset_erased!())
    }
}

impl<A: Clone + Send + Sync> AssetSnapshotStrategy<A> for BoxedAssetStorage {
    type AssetSnapshot = Box<A>;

    #[inline]
    fn get_snapshot(stored_asset: &mut Self::AssetStorage) -> Box<A> {
        Box::clone(
            stored_asset
                .as_ref()
                .unwrap_or_else(|| panic_asset_erased!()),
        )
    }
    #[inline]
    fn get_snapshot_erased(stored_asset: &mut Self::AssetStorage) -> Self::AssetSnapshot {
        stored_asset.take().unwrap_or_else(|| panic_asset_erased!())
    }
}
