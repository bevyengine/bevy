use crate::{Asset, AssetId, AssetIndexAllocator, AssetPath, InternalAssetId, UntypedAssetId};
use bevy_ecs::prelude::*;
use bevy_reflect::{FromReflect, Reflect, ReflectDeserialize, ReflectSerialize, Uuid};
use crossbeam_channel::{Receiver, Sender};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use std::{
    any::TypeId,
    hash::{Hash, Hasher},
    sync::Arc,
};

/// Provides [Handle] and [UntypedHandle] _for a specific asset type_
/// This should _only_ be used for one specific asset type
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
    pub(crate) fn get_handle(
        &self,
        id: InternalAssetId,
        asset_server_managed: bool,
        path: Option<AssetPath<'static>>,
    ) -> Arc<InternalAssetHandle> {
        Arc::new(InternalAssetHandle {
            id: id.untyped(self.type_id),
            drop_sender: self.drop_sender.clone(),
            path,
            asset_server_managed,
        })
    }

    pub(crate) fn reserve_handle_internal(
        &self,
        asset_server_managed: bool,
        path: Option<AssetPath<'static>>,
    ) -> Arc<InternalAssetHandle> {
        let index = self.allocator.reserve();
        self.get_handle(InternalAssetId::Index(index), asset_server_managed, path)
    }

    pub fn reserve_handle(&self) -> UntypedHandle {
        let index = self.allocator.reserve();
        UntypedHandle::Strong(self.get_handle(InternalAssetId::Index(index), false, None))
    }
}

#[derive(Debug)]
pub struct InternalAssetHandle {
    pub(crate) id: UntypedAssetId,
    pub(crate) asset_server_managed: bool,
    pub(crate) path: Option<AssetPath<'static>>,
    pub(crate) drop_sender: Sender<DropEvent>,
}

impl Drop for InternalAssetHandle {
    fn drop(&mut self) {
        if let Err(err) = self.drop_sender.send(DropEvent {
            id: self.id.internal(),
            asset_server_managed: self.asset_server_managed,
        }) {
            println!("Failed to send DropEvent for InternalAssetHandle {:?}", err);
        }
    }
}

#[derive(Component, Reflect, FromReflect)]
#[reflect_value(Component, PartialEq, Hash, Serialize, Deserialize)]
pub enum Handle<A: Asset> {
    Strong(Arc<InternalAssetHandle>),
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
    pub const fn weak_from_u128(value: u128) -> Self {
        Handle::Weak(AssetId::Uuid {
            uuid: Uuid::from_u128(value),
        })
    }
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

    #[inline]
    pub fn clone_weak(&self) -> Self {
        match self {
            Handle::Strong(handle) => Handle::Weak(handle.id.typed_unchecked::<A>()),
            Handle::Weak(id) => Handle::Weak(*id),
        }
    }

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
        let name = std::any::type_name::<A>().split("::").last().unwrap();
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

impl<A: Asset> Serialize for Handle<A> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct(std::any::type_name::<Self>(), 1)?;
        state.serialize_field("id", &self.id())?;
        state.end()
    }
}

impl<'de, A: Asset> Deserialize<'de> for Handle<A> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!("Give handle serialization a real think :)");
    }
}

#[derive(Clone)]
pub enum UntypedHandle {
    Strong(Arc<InternalAssetHandle>),
    Weak(UntypedAssetId),
}

impl UntypedHandle {
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

    #[inline]
    pub fn clone_weak(&self) -> UntypedHandle {
        match self {
            UntypedHandle::Strong(handle) => UntypedHandle::Weak(handle.id),
            UntypedHandle::Weak(id) => UntypedHandle::Weak(*id),
        }
    }

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

    /// Converts to a typed Handle. This _will not check if the target Handle type matches_.
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

    /// Converts to a typed Handle. This will panic if the internal TypeId does not match the given asset type `A`
    #[inline]
    pub fn typed<A: Asset>(self) -> Handle<A> {
        assert_eq!(
            self.type_id(),
            TypeId::of::<A>(),
            "The target Handle<A>'s TypeId does not match the TypeId of this UntypedHandle"
        );
        self.typed_unchecked()
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
