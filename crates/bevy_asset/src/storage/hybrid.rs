use alloc::sync::Arc;
use bevy_platform::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use core::{
    borrow::{Borrow, BorrowMut},
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use super::{AssetAsyncStrategy, AssetSnapshotStrategy, AssetStorageStrategy, AssetWriteStrategy};

macro_rules! panic_asset_erased {
    () => {
        panic!(
            "This {} asset has been erased",
            ::core::any::type_name::<Self>()
        )
    };
}

/// This storage strategy provides async read/write access to assets.
///
/// This is achieved by storing the asset on the stack by default, and upgrading to an [`Arc`] or [`RwLock`] when needed.
/// This approach allows assets that *don't* need to be shared across threads to not have to pay the potential performance cost
/// of locking. Also, for asset types that never request a `RwLock`, the compiler's optimizer should be able to optimize-away
/// the lock completely.
///
/// ## Ideal Usage
/// Best for heavy asset types (like images and meshes) that might need to be read or written in async contexts.
pub struct HybridAssetStorage;

pub enum HybridStorage<A> {
    /// Default storage state (low overhead)
    Owned(A),
    /// Referenced counting enabled (this asset is being shared across threads)
    UpgradedToArc(Arc<A>),
    /// This asset has been upgraded to a mutex so that it can be written in async contexts
    UpgradedToArcRwLock(Arc<RwLock<A>>),
    /// The asset's been removed
    Erased,
}

pub enum HybridStorageRef<'a, A> {
    Direct(&'a A),
    Guard(RwLockReadGuard<'a, A>),
}

impl<'a, A> Deref for HybridStorageRef<'a, A> {
    type Target = A;
    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Direct(asset) => asset,
            Self::Guard(asset) => asset.deref(),
        }
    }
}

impl<'a, A> Borrow<A> for HybridStorageRef<'a, A> {
    #[inline]
    fn borrow(&self) -> &A {
        match self {
            Self::Direct(asset) => asset,
            Self::Guard(asset) => asset.deref(),
        }
    }
}

pub enum HybridStorageMut<'a, A> {
    Direct(&'a mut A),
    Guard(RwLockWriteGuard<'a, A>),
}

impl<'a, A> Deref for HybridStorageMut<'a, A> {
    type Target = A;
    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Direct(asset) => asset,
            Self::Guard(asset) => asset.deref(),
        }
    }
}

impl<'a, A> DerefMut for HybridStorageMut<'a, A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Direct(asset) => asset,
            Self::Guard(asset) => asset.deref_mut(),
        }
    }
}

impl<'a, A> Borrow<A> for HybridStorageMut<'a, A> {
    #[inline]
    fn borrow(&self) -> &A {
        match self {
            Self::Direct(asset) => asset,
            Self::Guard(asset) => asset.deref(),
        }
    }
}

impl<'a, A> BorrowMut<A> for HybridStorageMut<'a, A> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut A {
        match self {
            Self::Direct(asset) => asset,
            Self::Guard(asset) => asset.deref_mut(),
        }
    }
}

impl<A: Send + Sync + Clone> AssetStorageStrategy<A> for HybridAssetStorage {
    type AssetStorage = HybridStorage<A>;

    type AssetRef<'a>
        = HybridStorageRef<'a, A>
    where
        A: 'a;

    #[inline]
    fn new(asset: A) -> Self::AssetStorage {
        HybridStorage::Owned(asset)
    }
    #[inline]
    fn get_ref(stored_asset: &Self::AssetStorage) -> Self::AssetRef<'_> {
        match stored_asset {
            HybridStorage::Owned(asset) => HybridStorageRef::Direct(asset),
            HybridStorage::UpgradedToArc(asset) => HybridStorageRef::Direct(asset),
            HybridStorage::UpgradedToArcRwLock(asset) => {
                HybridStorageRef::Guard(asset.read().unwrap())
            }
            HybridStorage::Erased => panic_asset_erased!(),
        }
    }
    #[inline]
    fn into_inner(stored_asset: Self::AssetStorage) -> Option<A> {
        match stored_asset {
            HybridStorage::Owned(asset) => Some(asset),
            HybridStorage::UpgradedToArc(asset) => Arc::into_inner(asset),
            HybridStorage::UpgradedToArcRwLock(asset) => Arc::into_inner(asset)?.into_inner().ok(),
            HybridStorage::Erased => None,
        }
    }
}

impl<A: Send + Sync + Clone> AssetWriteStrategy<A> for HybridAssetStorage {
    type AssetMut<'a>
        = HybridStorageMut<'a, A>
    where
        A: 'a;

    #[inline]
    fn get_mut<'a>(stored_asset: &'a mut Self::AssetStorage) -> Self::AssetMut<'a> {
        match stored_asset {
            HybridStorage::Owned(asset) => HybridStorageMut::Direct(asset),
            HybridStorage::UpgradedToArc(asset) => HybridStorageMut::Direct(Arc::make_mut(asset)),
            HybridStorage::UpgradedToArcRwLock(asset) => {
                HybridStorageMut::Guard(asset.write().unwrap())
            }
            HybridStorage::Erased => panic_asset_erased!(),
        }
    }
}

impl<A: Send + Sync + Clone> AssetAsyncStrategy<A> for HybridAssetStorage {
    fn get_arc(stored_asset: &mut Self::AssetStorage) -> Arc<A> {
        match stored_asset {
            HybridStorage::Owned(..) => {
                // Transition to UpgradedToArc (take the asset without cloning)
                let owned = core::mem::replace(stored_asset, HybridStorage::Erased);
                if let HybridStorage::Owned(asset) = owned {
                    let new_arc = Arc::new(asset);
                    *stored_asset = HybridStorage::UpgradedToArc(Arc::clone(&new_arc));
                    new_arc
                } else {
                    unreachable!()
                }
            }
            HybridStorage::UpgradedToArc(asset) => Arc::clone(asset),
            HybridStorage::UpgradedToArcRwLock(..) => {
                // Try to transition to UpgradedToArc if no outstanding locks exist
                let owned_stored_asset = core::mem::replace(stored_asset, HybridStorage::Erased);
                if let HybridStorage::UpgradedToArcRwLock(asset_lock) = owned_stored_asset {
                    match Arc::try_unwrap(asset_lock) {
                        Ok(asset_lock) => {
                            let new_arc = Arc::new(asset_lock.into_inner().unwrap());
                            *stored_asset = HybridStorage::UpgradedToArc(Arc::clone(&new_arc));
                            new_arc
                        }
                        Err(asset_lock) => {
                            // There's an outstanding lock, just clone the asset
                            let arc = Arc::new(asset_lock.read().unwrap().clone());
                            *stored_asset = HybridStorage::UpgradedToArcRwLock(asset_lock);
                            arc
                        }
                    }
                } else {
                    unreachable!()
                }
            }
            HybridStorage::Erased => panic_asset_erased!(),
        }
    }

    fn get_arc_rwlock(stored_asset: &mut Self::AssetStorage) -> Arc<RwLock<A>> {
        match stored_asset {
            HybridStorage::Owned(..) => {
                // Transition to UpgradedToArcRwLock
                let owned = core::mem::replace(stored_asset, HybridStorage::Erased);
                if let HybridStorage::Owned(asset) = owned {
                    let new_arc_rwlock = Arc::new(RwLock::new(asset));
                    *stored_asset = HybridStorage::UpgradedToArcRwLock(Arc::clone(&new_arc_rwlock));
                    new_arc_rwlock
                } else {
                    unreachable!()
                }
            }
            HybridStorage::UpgradedToArc(..) => {
                // Transition to UpgradedToArcRwLock
                let arc = core::mem::replace(stored_asset, HybridStorage::Erased);
                if let HybridStorage::UpgradedToArc(arc) = arc {
                    // Try to unwrap the Arc to get ownership, otherwise clone the inner value
                    let asset = Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone());
                    let new_arc_rwlock = Arc::new(RwLock::new(asset));
                    *stored_asset = HybridStorage::UpgradedToArcRwLock(Arc::clone(&new_arc_rwlock));
                    new_arc_rwlock
                } else {
                    unreachable!()
                }
            }
            HybridStorage::UpgradedToArcRwLock(asset) => Arc::clone(asset),
            HybridStorage::Erased => panic_asset_erased!(),
        }
    }
}

impl<A: Send + Sync + Clone> AssetSnapshotStrategy<A> for HybridAssetStorage {
    type AssetSnapshot = HybridSnapshot<A>;

    #[inline]
    fn get_snapshot(stored_asset: &mut Self::AssetStorage) -> HybridSnapshot<A> {
        HybridSnapshot::Arc(Self::get_arc(stored_asset))
    }
    fn get_snapshot_erased(stored_asset: &mut Self::AssetStorage) -> HybridSnapshot<A> {
        match stored_asset {
            HybridStorage::Owned(..) => {
                // Take ownership and transition to Erased
                let stored_asset = core::mem::replace(stored_asset, HybridStorage::Erased);
                if let HybridStorage::Owned(asset) = stored_asset {
                    HybridSnapshot::Owned(asset)
                } else {
                    unreachable!()
                }
            }
            HybridStorage::UpgradedToArc(..) => {
                // Try to take ownership and transition to Erased
                let stored_asset = core::mem::replace(stored_asset, HybridStorage::Erased);
                if let HybridStorage::UpgradedToArc(arc) = stored_asset {
                    match Arc::try_unwrap(arc) {
                        Ok(owned_asset) => HybridSnapshot::Owned(owned_asset),
                        Err(arc) => HybridSnapshot::Arc(arc),
                    }
                } else {
                    unreachable!()
                }
            }
            HybridStorage::UpgradedToArcRwLock(..) => {
                let stored_asset = core::mem::replace(stored_asset, HybridStorage::Erased);
                if let HybridStorage::UpgradedToArcRwLock(arc_rw_lock) = stored_asset {
                    match Arc::try_unwrap(arc_rw_lock) {
                        Ok(rw_lock) => HybridSnapshot::Owned(rw_lock.into_inner().unwrap()),
                        Err(arc_rw_lock) => {
                            HybridSnapshot::Owned(arc_rw_lock.read().unwrap().clone())
                        }
                    }
                } else {
                    unreachable!()
                }
            }
            HybridStorage::Erased => panic_asset_erased!(),
        }
    }
}

pub enum HybridSnapshot<A> {
    Arc(Arc<A>),
    Owned(A),
}

impl<A: Clone> HybridSnapshot<A> {
    #[inline]
    pub fn into_inner(self) -> A {
        match self {
            Self::Owned(asset) => asset,
            Self::Arc(asset) => Arc::try_unwrap(asset).unwrap_or_else(|arc| (*arc).clone()),
        }
    }
}

impl<A> Deref for HybridSnapshot<A> {
    type Target = A;
    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(asset) => asset,
            Self::Arc(asset) => asset.as_ref(),
        }
    }
}

impl<A> Debug for HybridSnapshot<A>
where
    A: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Owned(asset) => asset.fmt(f),
            Self::Arc(asset) => asset.fmt(f),
        }
    }
}

impl<A> Display for HybridSnapshot<A>
where
    A: Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Owned(asset) => asset.fmt(f),
            Self::Arc(asset) => asset.fmt(f),
        }
    }
}
