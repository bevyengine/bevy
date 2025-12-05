use core::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use super::{AssetSnapshotStrategy, AssetStorageStrategy, AssetWriteStrategy};

macro_rules! panic_asset_erased {
    () => {
        panic!(
            "This {} asset has been erased",
            ::core::any::type_name::<Self>()
        )
    };
}

pub struct StackAsset<A>(A);

impl<A> StackAsset<A> {
    #[inline]
    pub fn into_inner(self) -> A {
        self.0
    }
}

impl<A> Deref for StackAsset<A> {
    type Target = A;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A> DerefMut for StackAsset<A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<A> Borrow<A> for StackAsset<A> {
    #[inline]
    fn borrow(&self) -> &A {
        &self.0
    }
}

impl<A> BorrowMut<A> for StackAsset<A> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut A {
        &mut self.0
    }
}

impl<A> Debug for StackAsset<A>
where
    A: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

/// Best for light asset types that are cheap to clone (e.g. materials, config, etc)
pub struct StackAssetStorage;

impl<A: Send + Sync> AssetStorageStrategy<A> for StackAssetStorage {
    type AssetStorage = Option<StackAsset<A>>;

    type AssetRef<'a>
        = &'a A
    where
        A: 'a;

    #[inline]
    fn new(asset: A) -> Self::AssetStorage {
        Some(StackAsset(asset))
    }
    #[inline]
    fn get_ref<'a>(stored_asset: &'a Self::AssetStorage) -> &'a A {
        stored_asset
            .as_ref()
            .unwrap_or_else(|| panic_asset_erased!())
    }
    #[inline]
    fn into_inner(stored_asset: Self::AssetStorage) -> Option<A> {
        stored_asset.map(|stack_asset| stack_asset.0)
    }
}

impl<A: Send + Sync> AssetWriteStrategy<A> for StackAssetStorage {
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

impl<A: Clone + Send + Sync> AssetSnapshotStrategy<A> for StackAssetStorage {
    type AssetSnapshot = StackAsset<A>;

    #[inline]
    fn get_snapshot(stored_asset: &mut Self::AssetStorage) -> StackAsset<A> {
        StackAsset(
            stored_asset
                .as_ref()
                .unwrap_or_else(|| panic_asset_erased!())
                .0
                .clone(),
        )
    }
    #[inline]
    fn get_snapshot_erased(stored_asset: &mut Self::AssetStorage) -> StackAsset<A> {
        stored_asset.take().unwrap_or_else(|| panic_asset_erased!())
    }
}
