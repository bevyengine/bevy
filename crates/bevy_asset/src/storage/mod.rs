use crate::Asset;
use alloc::sync::Arc;
use bevy_platform::sync::RwLock;
use core::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut},
};

mod arced;
mod boxed;
mod hybrid;
mod stack;

pub use arced::*;
pub use boxed::*;
pub use hybrid::*;
pub use stack::*;

#[expect(type_alias_bounds, reason = "Type alias generics not yet stable")]
pub type StoredAsset<A: Asset> = <A::AssetStorage as AssetStorageStrategy<A>>::AssetStorage;

/// A reference to a stored asset. This can be dereferenced to `&A`.
#[expect(type_alias_bounds, reason = "Type alias generics not yet stable")]
pub type AssetRef<'a, A: Asset> = <A::AssetStorage as AssetStorageStrategy<A>>::AssetRef<'a>;

/// A mutable reference to a stored asset. This can be dereferenced to `&mut A`.
#[expect(type_alias_bounds, reason = "Type alias generics not yet stable")]
pub type AssetMut<'a, A: Asset> = <A::AssetStorage as AssetWriteStrategy<A>>::AssetMut<'a>;

/// A snapshot of an asset. This will be a clone of the asset `A`, or `Arc<A>`, depending on the storage strategy.
#[expect(type_alias_bounds, reason = "Type alias generics not yet stable")]
pub type AssetSnapshot<A: Asset> = <A::AssetStorage as AssetSnapshotStrategy<A>>::AssetSnapshot;

/// Defines how an asset `A` is stored internally.
pub trait AssetStorageStrategy<A> {
    type AssetStorage: Send + Sync;
    type AssetRef<'a>: Borrow<A> + Deref<Target = A>
    where
        Self: 'a,
        A: 'a;
    fn new(asset: A) -> Self::AssetStorage;

    /// Attempts to take ownership of the asset.
    ///
    /// This will return `None` if the asset has been erased, or if the asset has outstanding references.
    fn into_inner(stored_asset: Self::AssetStorage) -> Option<A>;

    /// Returns a reference to the asset.
    fn get_ref<'a>(stored_asset: &'a Self::AssetStorage) -> Self::AssetRef<'a>;
}

pub trait AssetWriteStrategy<A>: AssetStorageStrategy<A> {
    type AssetMut<'a>: BorrowMut<A> + Deref<Target = A> + DerefMut<Target = A>
    where
        Self: 'a,
        A: 'a;

    /// Returns a mutable reference to the asset.
    fn get_mut<'a>(stored_asset: &'a mut Self::AssetStorage) -> Self::AssetMut<'a>;
}

pub trait AssetSnapshotStrategy<A>: AssetStorageStrategy<A> {
    type AssetSnapshot: Send + Sync + Deref<Target = A>;

    /// Returns a snapshot of the asset, which is a clone of the asset `A` (or an `Arc<A>` clone, depending on the storage strategy).
    fn get_snapshot(stored_asset: &mut Self::AssetStorage) -> Self::AssetSnapshot;

    /// Instead of returning a clone of the asset or an Arc clone like [`crate::StoredAssetEntry::snapshot`],
    /// this will take ownership of the asset and put the entry in [`crate::Assets<A>`] into an erased state.
    ///
    /// Future attempts to get the asset will fail.
    fn get_snapshot_erased(stored_asset: &mut Self::AssetStorage) -> Self::AssetSnapshot;
}

pub trait AssetAsyncStrategy<A>: AssetStorageStrategy<A> {
    fn get_arc(stored_asset: &mut Self::AssetStorage) -> Arc<A>;
    fn get_arc_rwlock(stored_asset: &mut Self::AssetStorage) -> Arc<RwLock<A>>;
}
