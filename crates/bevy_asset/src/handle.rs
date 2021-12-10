use std::{
    cmp::Ordering,
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::{
    path::{AssetPath, AssetPathId},
    Asset, Assets,
};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_utils::Uuid;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};

/// A unique, stable asset id
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect,
)]
#[reflect_value(Serialize, Deserialize, PartialEq, Hash)]
pub enum HandleId {
    Id(Uuid, u64),
    AssetPathId(AssetPathId),
}

impl From<AssetPathId> for HandleId {
    fn from(value: AssetPathId) -> Self {
        HandleId::AssetPathId(value)
    }
}

impl<'a> From<AssetPath<'a>> for HandleId {
    fn from(value: AssetPath<'a>) -> Self {
        HandleId::AssetPathId(AssetPathId::from(value))
    }
}

impl HandleId {
    #[inline]
    pub fn random<T: Asset>() -> Self {
        HandleId::Id(T::TYPE_UUID, rand::random())
    }

    #[inline]
    pub fn default<T: Asset>() -> Self {
        HandleId::Id(T::TYPE_UUID, 0)
    }

    #[inline]
    pub const fn new(type_uuid: Uuid, id: u64) -> Self {
        HandleId::Id(type_uuid, id)
    }
}

/// A handle into a specific Asset of type `T`
///
/// Handles contain a unique id that corresponds to a specific asset in the [Assets](crate::Assets)
/// collection.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Handle<T>
where
    T: Asset,
{
    pub id: HandleId,
    #[reflect(ignore)]
    handle_type: HandleType,
    #[reflect(ignore)]
    // NOTE: PhantomData<fn() -> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> T>,
}

enum HandleType {
    Weak,
    Strong(Sender<RefChange>),
}

impl Debug for HandleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleType::Weak => f.write_str("Weak"),
            HandleType::Strong(_) => f.write_str("Strong"),
        }
    }
}

impl<T: Asset> Handle<T> {
    pub(crate) fn strong(id: HandleId, ref_change_sender: Sender<RefChange>) -> Self {
        ref_change_sender.send(RefChange::Increment(id)).unwrap();
        Self {
            id,
            handle_type: HandleType::Strong(ref_change_sender),
            marker: PhantomData,
        }
    }

    #[inline]
    pub fn weak(id: HandleId) -> Self {
        Self {
            id,
            handle_type: HandleType::Weak,
            marker: PhantomData,
        }
    }

    pub fn as_weak<U: Asset>(&self) -> Handle<U> {
        Handle {
            id: self.id,
            handle_type: HandleType::Weak,
            marker: PhantomData,
        }
    }

    pub fn is_weak(&self) -> bool {
        matches!(self.handle_type, HandleType::Weak)
    }

    pub fn is_strong(&self) -> bool {
        matches!(self.handle_type, HandleType::Strong(_))
    }

    pub fn make_strong(&mut self, assets: &mut Assets<T>) {
        if self.is_strong() {
            return;
        }
        let sender = assets.ref_change_sender.clone();
        sender.send(RefChange::Increment(self.id)).unwrap();
        self.handle_type = HandleType::Strong(sender);
    }

    #[inline]
    pub fn clone_weak(&self) -> Self {
        Handle::weak(self.id)
    }

    pub fn clone_untyped(&self) -> HandleUntyped {
        match &self.handle_type {
            HandleType::Strong(sender) => HandleUntyped::strong(self.id, sender.clone()),
            HandleType::Weak => HandleUntyped::weak(self.id),
        }
    }

    pub fn clone_weak_untyped(&self) -> HandleUntyped {
        HandleUntyped::weak(self.id)
    }
}

impl<T: Asset> Drop for Handle<T> {
    fn drop(&mut self) {
        match self.handle_type {
            HandleType::Strong(ref sender) => {
                // ignore send errors because this means the channel is shut down / the game has
                // stopped
                let _ = sender.send(RefChange::Decrement(self.id));
            }
            HandleType::Weak => {}
        }
    }
}

impl<T: Asset> From<Handle<T>> for HandleId {
    fn from(value: Handle<T>) -> Self {
        value.id
    }
}

impl From<HandleUntyped> for HandleId {
    fn from(value: HandleUntyped) -> Self {
        value.id
    }
}

impl From<&str> for HandleId {
    fn from(value: &str) -> Self {
        AssetPathId::from(value).into()
    }
}

impl From<&String> for HandleId {
    fn from(value: &String) -> Self {
        AssetPathId::from(value).into()
    }
}

impl From<String> for HandleId {
    fn from(value: String) -> Self {
        AssetPathId::from(&value).into()
    }
}

impl<T: Asset> From<&Handle<T>> for HandleId {
    fn from(value: &Handle<T>) -> Self {
        value.id
    }
}

impl<T: Asset> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.id, state);
    }
}

impl<T: Asset> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Asset> Eq for Handle<T> {}

impl<T: Asset> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl<T: Asset> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: Asset> Default for Handle<T> {
    fn default() -> Self {
        Handle::weak(HandleId::default::<T>())
    }
}

impl<T: Asset> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let name = std::any::type_name::<T>().split("::").last().unwrap();
        write!(f, "{:?}Handle<{}>({:?})", self.handle_type, name, self.id)
    }
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        match self.handle_type {
            HandleType::Strong(ref sender) => Handle::strong(self.id, sender.clone()),
            HandleType::Weak => Handle::weak(self.id),
        }
    }
}

/// A non-generic version of [Handle]
///
/// This allows handles to be mingled in a cross asset context. For example, storing `Handle<A>` and
/// `Handle<B>` in the same `HashSet<HandleUntyped>`.
#[derive(Debug)]
pub struct HandleUntyped {
    pub id: HandleId,
    handle_type: HandleType,
}

impl HandleUntyped {
    pub const fn weak_from_u64(uuid: Uuid, id: u64) -> Self {
        Self {
            id: HandleId::new(uuid, id),
            handle_type: HandleType::Weak,
        }
    }

    pub(crate) fn strong(id: HandleId, ref_change_sender: Sender<RefChange>) -> Self {
        ref_change_sender.send(RefChange::Increment(id)).unwrap();
        Self {
            id,
            handle_type: HandleType::Strong(ref_change_sender),
        }
    }

    pub fn weak(id: HandleId) -> Self {
        Self {
            id,
            handle_type: HandleType::Weak,
        }
    }

    pub fn clone_weak(&self) -> HandleUntyped {
        HandleUntyped::weak(self.id)
    }

    pub fn is_weak(&self) -> bool {
        matches!(self.handle_type, HandleType::Weak)
    }

    pub fn is_strong(&self) -> bool {
        matches!(self.handle_type, HandleType::Strong(_))
    }

    pub fn typed<T: Asset>(mut self) -> Handle<T> {
        if let HandleId::Id(type_uuid, _) = self.id {
            if T::TYPE_UUID != type_uuid {
                panic!("Attempted to convert handle to invalid type.");
            }
        }
        let handle_type = match &self.handle_type {
            HandleType::Strong(sender) => HandleType::Strong(sender.clone()),
            HandleType::Weak => HandleType::Weak,
        };
        // ensure we don't send the RefChange event when "self" is dropped
        self.handle_type = HandleType::Weak;
        Handle {
            handle_type,
            id: self.id,
            marker: PhantomData::default(),
        }
    }
}

impl Drop for HandleUntyped {
    fn drop(&mut self) {
        match self.handle_type {
            HandleType::Strong(ref sender) => {
                // ignore send errors because this means the channel is shut down / the game has
                // stopped
                let _ = sender.send(RefChange::Decrement(self.id));
            }
            HandleType::Weak => {}
        }
    }
}

impl From<&HandleUntyped> for HandleId {
    fn from(value: &HandleUntyped) -> Self {
        value.id
    }
}

impl Hash for HandleUntyped {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.id, state);
    }
}

impl PartialEq for HandleUntyped {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for HandleUntyped {}

impl Clone for HandleUntyped {
    fn clone(&self) -> Self {
        match self.handle_type {
            HandleType::Strong(ref sender) => HandleUntyped::strong(self.id, sender.clone()),
            HandleType::Weak => HandleUntyped::weak(self.id),
        }
    }
}

pub(crate) enum RefChange {
    Increment(HandleId),
    Decrement(HandleId),
}

#[derive(Clone)]
pub(crate) struct RefChangeChannel {
    pub sender: Sender<RefChange>,
    pub receiver: Receiver<RefChange>,
}

impl Default for RefChangeChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        RefChangeChannel { sender, receiver }
    }
}
