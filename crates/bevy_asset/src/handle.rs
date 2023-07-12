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
use bevy_reflect::{std_traits::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_utils::Uuid;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};

/// A unique, stable asset id.
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect,
)]
#[reflect_value(Serialize, Deserialize, PartialEq, Hash)]
pub enum HandleId {
    /// A handle id of a loaded asset.
    Id(Uuid, u64),

    /// A handle id of a pending asset.
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
    /// Creates a random id for an asset of type `T`.
    #[inline]
    pub fn random<T: Asset>() -> Self {
        HandleId::Id(T::TYPE_UUID, fastrand::u64(..))
    }

    /// Creates the default id for an asset of type `T`.
    #[inline]
    #[allow(clippy::should_implement_trait)] // `Default` is not implemented for `HandleId`, the default value depends on the asset type
    pub fn default<T: Asset>() -> Self {
        HandleId::Id(T::TYPE_UUID, 0)
    }

    /// Creates an arbitrary asset id without an explicit type bound.
    #[inline]
    pub const fn new(type_uuid: Uuid, id: u64) -> Self {
        HandleId::Id(type_uuid, id)
    }
}

/// A handle into a specific [`Asset`] of type `T`.
///
/// Handles contain a unique id that corresponds to a specific asset in the [`Assets`] collection.
///
/// # Accessing the Asset
///
/// A handle is _not_ the asset itself, but should be seen as a pointer to the asset. Modifying a
/// handle's `id` only modifies which asset is being pointed to. To get the actual asset, try using
/// [`Assets::get`] or [`Assets::get_mut`].
///
/// # Strong and Weak
///
/// A handle can be either "Strong" or "Weak". Simply put: Strong handles keep the asset loaded,
/// while Weak handles do not affect the loaded status of assets. This is due to a type of
/// _reference counting_. When the number of Strong handles that exist for any given asset reach
/// zero, the asset is dropped and becomes unloaded. In some cases, you might want a reference to an
/// asset but don't want to take the responsibility of keeping it loaded that comes with a Strong handle.
/// This is where a Weak handle can be very useful.
///
/// For example, imagine you have a `Sprite` component and a `Collider` component. The `Collider` uses
/// the `Sprite`'s image size to check for collisions. It does so by keeping a Weak copy of the
/// `Sprite`'s Strong handle to the image asset.
///
/// If the `Sprite` is removed, its Strong handle to the image is dropped with it. And since it was the
/// only Strong handle for that asset, the asset is unloaded. Our `Collider` component still has a Weak
/// handle to the unloaded asset, but it will not be able to retrieve the image data, resulting in
/// collisions no longer being detected for that entity.
///
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub struct Handle<T>
where
    T: Asset,
{
    id: HandleId,
    #[reflect(ignore)]
    handle_type: HandleType,
    #[reflect(ignore)]
    // NOTE: PhantomData<fn() -> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> T>,
}

#[derive(Default)]
enum HandleType {
    #[default]
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

    /// Creates a weak handle into an Asset identified by `id`.
    #[inline]
    pub fn weak(id: HandleId) -> Self {
        Self {
            id,
            handle_type: HandleType::Weak,
            marker: PhantomData,
        }
    }

    /// The ID of the asset as contained within its respective [`Assets`] collection.
    #[inline]
    pub fn id(&self) -> HandleId {
        self.id
    }

    /// Recasts this handle as a weak handle of an Asset `U`.
    pub fn cast_weak<U: Asset>(&self) -> Handle<U> {
        let id = if let HandleId::Id(_, id) = self.id {
            HandleId::Id(U::TYPE_UUID, id)
        } else {
            self.id
        };

        Handle {
            id,
            handle_type: HandleType::Weak,
            marker: PhantomData,
        }
    }

    /// Returns `true` if this is a weak handle.
    pub fn is_weak(&self) -> bool {
        matches!(self.handle_type, HandleType::Weak)
    }

    /// Returns `true` if this is a strong handle.
    pub fn is_strong(&self) -> bool {
        matches!(self.handle_type, HandleType::Strong(_))
    }

    /// Makes this handle Strong if it wasn't already.
    ///
    /// This method requires the corresponding [`Assets`](crate::Assets) collection.
    pub fn make_strong(&mut self, assets: &Assets<T>) {
        if self.is_strong() {
            return;
        }
        let sender = assets.ref_change_sender.clone();
        sender.send(RefChange::Increment(self.id)).unwrap();
        self.handle_type = HandleType::Strong(sender);
    }

    /// Creates a weak copy of this handle.
    #[inline]
    #[must_use]
    pub fn clone_weak(&self) -> Self {
        Self::weak(self.id)
    }

    /// Creates an untyped copy of this handle.
    pub fn clone_untyped(&self) -> HandleUntyped {
        match &self.handle_type {
            HandleType::Strong(sender) => HandleUntyped::strong(self.id, sender.clone()),
            HandleType::Weak => HandleUntyped::weak(self.id),
        }
    }

    /// Creates a weak, untyped copy of this handle.
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
        Some(self.cmp(other))
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
        write!(f, "{:?}Handle<{name}>({:?})", self.handle_type, self.id)
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

/// A non-generic version of [`Handle`].
///
/// This allows handles to be mingled in a cross asset context. For example, storing `Handle<A>` and
/// `Handle<B>` in the same `HashSet<HandleUntyped>`.
///
/// To convert back to a typed handle, use the [typed](HandleUntyped::typed) method.
#[derive(Debug)]
pub struct HandleUntyped {
    id: HandleId,
    handle_type: HandleType,
}

impl HandleUntyped {
    /// Creates a weak untyped handle with an arbitrary id.
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

    /// Create a weak, untyped handle into an Asset identified by `id`.
    pub fn weak(id: HandleId) -> Self {
        Self {
            id,
            handle_type: HandleType::Weak,
        }
    }

    /// The ID of the asset.
    #[inline]
    pub fn id(&self) -> HandleId {
        self.id
    }

    /// Creates a weak copy of this handle.
    #[must_use]
    pub fn clone_weak(&self) -> Self {
        Self::weak(self.id)
    }

    /// Returns `true` if this is a weak handle.
    pub fn is_weak(&self) -> bool {
        matches!(self.handle_type, HandleType::Weak)
    }

    /// Returns `true` if this is a strong handle.
    pub fn is_strong(&self) -> bool {
        matches!(self.handle_type, HandleType::Strong(_))
    }

    /// Create a weak typed [`Handle`] from this handle.
    ///
    /// If this handle is strong and dropped, there is no guarantee that the asset
    /// will still be available (if only the returned handle is kept)
    pub fn typed_weak<T: Asset>(&self) -> Handle<T> {
        self.clone_weak().typed()
    }

    /// Converts this handle into a typed [`Handle`] of an [`Asset`] `T`.
    ///
    /// The new handle will maintain the Strong or Weak status of the current handle.
    ///
    /// # Panics
    ///
    /// Will panic if type `T` doesn't match this handle's actual asset type.
    pub fn typed<T: Asset>(mut self) -> Handle<T> {
        if let HandleId::Id(type_uuid, _) = self.id {
            assert!(
                T::TYPE_UUID == type_uuid,
                "Attempted to convert handle to invalid type."
            );
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

impl<A: Asset> From<Handle<A>> for HandleUntyped {
    fn from(mut handle: Handle<A>) -> Self {
        let handle_type = std::mem::replace(&mut handle.handle_type, HandleType::Weak);
        HandleUntyped {
            id: handle.id,
            handle_type,
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
