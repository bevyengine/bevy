use crate::{
    meta::MetaTransform, Asset, AssetId, AssetIndexAllocator, AssetPath, InternalAssetId,
    UntypedAssetId,
};
use bevy_ecs::prelude::*;
use bevy_reflect::{Reflect, TypePath, Uuid};
use bevy_utils::get_short_name;
use crossbeam_channel::{Receiver, Sender};
use std::{
    any::TypeId,
    hash::{Hash, Hasher},
    sync::Arc,
};

/// Provides [`Handle`] and [`UntypedHandle`] _for a specific asset type_.
/// This should _only_ be used for one specific asset type.
#[derive(Clone)]
pub struct AssetHandleProvider {
    pub(crate) allocator: Arc<AssetIndexAllocator>,
    pub(crate) drop_sender: Sender<DropEvent>,
    pub(crate) drop_receiver: Receiver<DropEvent>,
    pub(crate) type_id: TypeId,
}

pub(crate) struct DropEvent {
    pub(crate) id: InternalAssetId,
    pub(crate) asset_server_managed: bool,
}

impl AssetHandleProvider {
    pub(crate) fn new(type_id: TypeId, allocator: Arc<AssetIndexAllocator>) -> Self {
        let (drop_sender, drop_receiver) = crossbeam_channel::unbounded();
        Self {
            type_id,
            allocator,
            drop_sender,
            drop_receiver,
        }
    }

    /// Reserves a new strong [`UntypedHandle`] (with a new [`UntypedAssetId`]). The stored [`Asset`] [`TypeId`] in the
    /// [`UntypedHandle`] will match the [`Asset`] [`TypeId`] assigned to this [`AssetHandleProvider`].
    pub fn reserve_handle(&self) -> UntypedHandle {
        let index = self.allocator.reserve();
        UntypedHandle::Strong(self.get_handle(InternalAssetId::Index(index), false, None, None))
    }

    pub(crate) fn get_handle(
        &self,
        id: InternalAssetId,
        asset_server_managed: bool,
        path: Option<AssetPath<'static>>,
        meta_transform: Option<MetaTransform>,
    ) -> Arc<StrongHandle> {
        Arc::new(StrongHandle {
            id: id.untyped(self.type_id),
            drop_sender: self.drop_sender.clone(),
            meta_transform,
            path,
            asset_server_managed,
        })
    }

    pub(crate) fn reserve_handle_internal(
        &self,
        asset_server_managed: bool,
        path: Option<AssetPath<'static>>,
        meta_transform: Option<MetaTransform>,
    ) -> Arc<StrongHandle> {
        let index = self.allocator.reserve();
        self.get_handle(
            InternalAssetId::Index(index),
            asset_server_managed,
            path,
            meta_transform,
        )
    }
}

/// The internal "strong" [`Asset`] handle storage for [`Handle::Strong`] and [`UntypedHandle::Strong`]. When this is dropped,
/// the [`Asset`] will be freed. It also stores some asset metadata for easy access from handles.
#[derive(TypePath)]
pub struct StrongHandle {
    pub(crate) id: UntypedAssetId,
    pub(crate) asset_server_managed: bool,
    pub(crate) path: Option<AssetPath<'static>>,
    /// Modifies asset meta. This is stored on the handle because it is:
    /// 1. configuration tied to the lifetime of a specific asset load
    /// 2. configuration that must be repeatable when the asset is hot-reloaded
    pub(crate) meta_transform: Option<MetaTransform>,
    pub(crate) drop_sender: Sender<DropEvent>,
}

impl Drop for StrongHandle {
    fn drop(&mut self) {
        if let Err(err) = self.drop_sender.send(DropEvent {
            id: self.id.internal(),
            asset_server_managed: self.asset_server_managed,
        }) {
            println!("Failed to send DropEvent for StrongHandle {:?}", err);
        }
    }
}

impl std::fmt::Debug for StrongHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrongHandle")
            .field("id", &self.id)
            .field("asset_server_managed", &self.asset_server_managed)
            .field("path", &self.path)
            .field("drop_sender", &self.drop_sender)
            .finish()
    }
}

/// A strong or weak handle to a specific [`Asset`]. If a [`Handle`] is [`Handle::Strong`], the [`Asset`] will be kept
/// alive until the [`Handle`] is dropped. If a [`Handle`] is [`Handle::Weak`], it does not necessarily reference a live [`Asset`],
/// nor will it keep assets alive.
///
/// [`Handle`] can be cloned. If a [`Handle::Strong`] is cloned, the referenced [`Asset`] will not be freed until _all_ instances
/// of the [`Handle`] are dropped.
///
/// [`Handle::Strong`] also provides access to useful [`Asset`] metadata, such as the [`AssetPath`] (if it exists).
#[derive(Component, Reflect)]
#[reflect(Component)]
pub enum Handle<A: Asset> {
    /// A "strong" reference to a live (or loading) [`Asset`]. If a [`Handle`] is [`Handle::Strong`], the [`Asset`] will be kept
    /// alive until the [`Handle`] is dropped. Strong handles also provide access to additional asset metadata.  
    Strong(Arc<StrongHandle>),
    /// A "weak" reference to an [`Asset`]. If a [`Handle`] is [`Handle::Weak`], it does not necessarily reference a live [`Asset`],
    /// nor will it keep assets alive.
    Weak(AssetId<A>),
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        match self {
            Handle::Strong(handle) => Handle::Strong(handle.clone()),
            Handle::Weak(id) => Handle::Weak(*id),
        }
    }
}

impl<A: Asset> Handle<A> {
    /// Create a new [`Handle::Weak`] with the given [`u128`] encoding of a [`Uuid`].
    pub const fn weak_from_u128(value: u128) -> Self {
        Handle::Weak(AssetId::Uuid {
            uuid: Uuid::from_u128(value),
        })
    }

    /// Returns the [`AssetId`] of this [`Asset`].
    #[inline]
    pub fn id(&self) -> AssetId<A> {
        match self {
            Handle::Strong(handle) => handle.id.typed_unchecked(),
            Handle::Weak(id) => *id,
        }
    }

    /// Returns the path if this is (1) a strong handle and (2) the asset has a path
    #[inline]
    pub fn path(&self) -> Option<&AssetPath<'static>> {
        match self {
            Handle::Strong(handle) => handle.path.as_ref(),
            Handle::Weak(_) => None,
        }
    }

    /// Returns `true` if this is a weak handle.
    #[inline]
    pub fn is_weak(&self) -> bool {
        matches!(self, Handle::Weak(_))
    }

    /// Returns `true` if this is a strong handle.
    #[inline]
    pub fn is_strong(&self) -> bool {
        matches!(self, Handle::Strong(_))
    }

    /// Creates a [`Handle::Weak`] clone of this [`Handle`], which will not keep the referenced [`Asset`] alive.
    #[inline]
    pub fn clone_weak(&self) -> Self {
        match self {
            Handle::Strong(handle) => Handle::Weak(handle.id.typed_unchecked::<A>()),
            Handle::Weak(id) => Handle::Weak(*id),
        }
    }

    /// Converts this [`Handle`] to an "untyped" / "generic-less" [`UntypedHandle`], which stores the [`Asset`] type information
    /// _inside_ [`UntypedHandle`]. This will return [`UntypedHandle::Strong`] for [`Handle::Strong`] and [`UntypedHandle::Weak`] for
    /// [`Handle::Weak`].  
    #[inline]
    pub fn untyped(self) -> UntypedHandle {
        match self {
            Handle::Strong(handle) => UntypedHandle::Strong(handle),
            Handle::Weak(id) => UntypedHandle::Weak(id.untyped()),
        }
    }
}

impl<A: Asset> Default for Handle<A> {
    fn default() -> Self {
        Handle::Weak(AssetId::default())
    }
}

impl<A: Asset> std::fmt::Debug for Handle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = get_short_name(std::any::type_name::<A>());
        match self {
            Handle::Strong(handle) => {
                write!(
                    f,
                    "StrongHandle<{name}>{{ id: {:?}, path: {:?} }}",
                    handle.id.internal(),
                    handle.path
                )
            }
            Handle::Weak(id) => write!(f, "WeakHandle<{name}>({:?})", id.internal()),
        }
    }
}

impl<A: Asset> Hash for Handle<A> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.id(), state);
    }
}

impl<A: Asset> PartialOrd for Handle<A> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.id().cmp(&other.id()))
    }
}

impl<A: Asset> Ord for Handle<A> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id().cmp(&other.id())
    }
}

impl<A: Asset> PartialEq for Handle<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl<A: Asset> Eq for Handle<A> {}

/// An untyped variant of [`Handle`], which internally stores the [`Asset`] type information at runtime
/// as a [`TypeId`] instead of encoding it in the compile-time type. This allows handles across [`Asset`] types
/// to be stored together and compared.
///
/// See [`Handle`] for more information.
#[derive(Clone)]
pub enum UntypedHandle {
    Strong(Arc<StrongHandle>),
    Weak(UntypedAssetId),
}

impl UntypedHandle {
    /// Returns the [`UntypedAssetId`] for the referenced asset.
    #[inline]
    pub fn id(&self) -> UntypedAssetId {
        match self {
            UntypedHandle::Strong(handle) => handle.id,
            UntypedHandle::Weak(id) => *id,
        }
    }

    /// Returns the path if this is (1) a strong handle and (2) the asset has a path
    #[inline]
    pub fn path(&self) -> Option<&AssetPath<'static>> {
        match self {
            UntypedHandle::Strong(handle) => handle.path.as_ref(),
            UntypedHandle::Weak(_) => None,
        }
    }

    /// Creates an [`UntypedHandle::Weak`] clone of this [`UntypedHandle`], which will not keep the referenced [`Asset`] alive.
    #[inline]
    pub fn clone_weak(&self) -> UntypedHandle {
        match self {
            UntypedHandle::Strong(handle) => UntypedHandle::Weak(handle.id),
            UntypedHandle::Weak(id) => UntypedHandle::Weak(*id),
        }
    }

    /// Returns the [`TypeId`] of the referenced [`Asset`].
    #[inline]
    pub fn type_id(&self) -> TypeId {
        match self {
            UntypedHandle::Strong(handle) => handle.id.type_id(),
            UntypedHandle::Weak(id) => id.type_id(),
        }
    }

    /// Converts to a typed Handle. This _will not check if the target Handle type matches_.
    #[inline]
    pub fn typed_unchecked<A: Asset>(self) -> Handle<A> {
        match self {
            UntypedHandle::Strong(handle) => Handle::Strong(handle),
            UntypedHandle::Weak(id) => Handle::Weak(id.typed_unchecked::<A>()),
        }
    }

    /// Converts to a typed Handle. This will check the type when compiled with debug asserts, but it
    ///  _will not check if the target Handle type matches in release builds_. Use this as an optimization
    /// when you want some degree of validation at dev-time, but you are also very certain that the type
    /// actually matches.
    #[inline]
    pub fn typed_debug_checked<A: Asset>(self) -> Handle<A> {
        debug_assert_eq!(
            self.type_id(),
            TypeId::of::<A>(),
            "The target Handle<A>'s TypeId does not match the TypeId of this UntypedHandle"
        );
        match self {
            UntypedHandle::Strong(handle) => Handle::Strong(handle),
            UntypedHandle::Weak(id) => Handle::Weak(id.typed_unchecked::<A>()),
        }
    }

    /// Converts to a typed Handle. This will panic if the internal [`TypeId`] does not match the given asset type `A`
    #[inline]
    pub fn typed<A: Asset>(self) -> Handle<A> {
        assert_eq!(
            self.type_id(),
            TypeId::of::<A>(),
            "The target Handle<A>'s TypeId does not match the TypeId of this UntypedHandle"
        );
        self.typed_unchecked()
    }

    /// The "meta transform" for the strong handle. This will only be [`Some`] if the handle is strong and there is a meta transform
    /// associated with it.
    #[inline]
    pub fn meta_transform(&self) -> Option<&MetaTransform> {
        match self {
            UntypedHandle::Strong(handle) => handle.meta_transform.as_ref(),
            UntypedHandle::Weak(_) => None,
        }
    }
}

impl PartialEq for UntypedHandle {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id() && self.type_id() == other.type_id()
    }
}

impl Eq for UntypedHandle {}

impl Hash for UntypedHandle {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
        self.type_id().hash(state);
    }
}

impl std::fmt::Debug for UntypedHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UntypedHandle::Strong(handle) => {
                write!(
                    f,
                    "StrongHandle{{ type_id: {:?}, id: {:?}, path: {:?} }}",
                    handle.id.type_id(),
                    handle.id.internal(),
                    handle.path
                )
            }
            UntypedHandle::Weak(id) => write!(
                f,
                "WeakHandle{{ type_id: {:?}, id: {:?} }}",
                id.type_id(),
                id.internal()
            ),
        }
    }
}
