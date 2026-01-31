use crate::{meta::MetaTransform, Asset, AssetEntity, AssetId, AssetPath, UntypedAssetId};
use alloc::sync::{Arc, Weak};
use bevy_ecs::{component::Component, resource::Resource};
use bevy_reflect::{std_traits::ReflectDefault, Reflect, TypePath};
use core::{
    any::TypeId,
    hash::{Hash, Hasher},
    marker::PhantomData,
};
use crossbeam_channel::{bounded, never, Receiver, Sender};
use disqualified::ShortName;
use thiserror::Error;
use uuid::Uuid;

#[derive(Component)]
pub struct AssetSelfHandle(pub(crate) Weak<StrongHandle>);

impl AssetSelfHandle {
    pub fn upgrade<A: Asset>(&self) -> Result<Handle<A>, HandleUpgradeError> {
        let handle = self
            .upgrade_untyped()
            .ok_or(HandleUpgradeError::ExpiredHandle)?;
        Ok(handle.try_typed()?)
    }

    pub fn upgrade_untyped(&self) -> Option<UntypedHandle> {
        self.0.upgrade().map(UntypedHandle::Strong)
    }
}

#[derive(Error, Debug, Clone)]
pub enum HandleUpgradeError {
    #[error("The underlying handle has been dropped, so the handle can't be upgraded")]
    ExpiredHandle,
    #[error(transparent)]
    WrongType(#[from] UntypedAssetConversionError),
}

/// Provides [`Handle`] and [`UntypedHandle`]s for an entity.
#[derive(Resource, Clone)]
pub(crate) struct AssetHandleProvider {
    // TODO: We only need the TypeId for sending AssetEvent::Unused. Remove the TypeId once we're
    // using events.
    pub(crate) drop_sender: Sender<(AssetEntity, TypeId)>,
    pub(crate) drop_receiver: Receiver<(AssetEntity, TypeId)>,
}

impl AssetHandleProvider {
    /// Creates a fake provider, for use in cases where we don't need real handles.
    pub(crate) fn fake() -> Self {
        // Create a totally fake channel. The sender channel is already closed, and the receiver is
        // just the `never` channel.
        let (sender, _) = bounded(0);
        let receiver = never();
        Self {
            drop_sender: sender,
            drop_receiver: receiver,
        }
    }

    /// Creates a new instance of a handle provider.
    pub(crate) fn new() -> Self {
        let (drop_sender, drop_receiver) = crossbeam_channel::unbounded();
        Self {
            drop_sender,
            drop_receiver,
        }
    }

    /// Creates a handle with all its asset's details provided.
    pub(crate) fn create_handle(
        &self,
        entity: AssetEntity,
        type_id: TypeId,
        path: Option<AssetPath<'static>>,
        meta_transform: Option<MetaTransform>,
    ) -> Arc<StrongHandle> {
        Arc::new(StrongHandle {
            entity,
            type_id,
            meta_transform,
            path,
            drop_sender: self.drop_sender.clone(),
        })
    }
}

/// The internal "strong" [`Asset`] handle storage for [`Handle::Strong`] and [`UntypedHandle::Strong`]. When this is dropped,
/// the [`Asset`] will be freed. It also stores some asset metadata for easy access from handles.
#[derive(TypePath)]
pub struct StrongHandle {
    pub(crate) entity: AssetEntity,
    pub(crate) type_id: TypeId,
    pub(crate) path: Option<AssetPath<'static>>,
    /// Modifies asset meta. This is stored on the handle because it is:
    /// 1. configuration tied to the lifetime of a specific asset load
    /// 2. configuration that must be repeatable when the asset is hot-reloaded
    pub(crate) meta_transform: Option<MetaTransform>,
    pub(crate) drop_sender: Sender<(AssetEntity, TypeId)>,
}

impl Drop for StrongHandle {
    fn drop(&mut self) {
        let _ = self.drop_sender.send((self.entity, self.type_id));
    }
}

impl core::fmt::Debug for StrongHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StrongHandle")
            .field("entity", &self.entity)
            .field("type_id", &self.type_id)
            .field("path", &self.path)
            .field("drop_sender", &self.drop_sender)
            .finish()
    }
}

/// A handle to an asset entity whose type information only exists at runtime.
///
/// Unlike [`UntypedHandle`], this handle is guaranteed to be pointed at an entity, and not at a
/// UUID.
#[derive(Clone, Debug)]
pub struct UntypedEntityHandle(pub(crate) Arc<StrongHandle>);

impl UntypedEntityHandle {
    /// Returns the asset entity this handle references.
    #[inline]
    pub fn entity(&self) -> AssetEntity {
        self.0.entity
    }

    /// Returns the [`UntypedAssetId`] for this handle.
    #[inline]
    pub fn id(&self) -> UntypedAssetId {
        UntypedAssetId::Entity {
            type_id: self.0.type_id,
            entity: self.0.entity,
        }
    }

    /// Returns the type ID of the asset being referenced.
    pub fn type_id(&self) -> TypeId {
        self.0.type_id
    }

    /// Returns the path if the asset has a path.
    #[inline]
    pub fn path(&self) -> Option<&AssetPath<'static>> {
        self.0.path.as_ref()
    }

    /// Converts to a typed Handle. This _will not check if the target Handle type matches_.
    #[inline]
    pub fn typed_unchecked<A: Asset>(self) -> EntityHandle<A> {
        EntityHandle(self.0, PhantomData)
    }

    /// Converts to a typed Handle. This will check the type when compiled with debug asserts, but it
    ///  _will not check if the target Handle type matches in release builds_. Use this as an optimization
    /// when you want some degree of validation at dev-time, but you are also very certain that the type
    /// actually matches.
    #[inline]
    pub fn typed_debug_checked<A: Asset>(self) -> EntityHandle<A> {
        debug_assert_eq!(
            self.0.type_id,
            TypeId::of::<A>(),
            "The target EntityHandle<A>'s TypeId does not match the TypeId of this UntypedEntityHandle"
        );
        self.typed_unchecked()
    }

    /// Converts to a typed Handle. This will panic if the internal [`TypeId`] does not match the given asset type `A`
    #[inline]
    pub fn typed<A: Asset>(self) -> EntityHandle<A> {
        let Ok(handle) = self.try_typed() else {
            panic!(
                "The target EntityHandle<{}>'s TypeId does not match the TypeId of this UntypedEntityHandle",
                core::any::type_name::<A>()
            )
        };

        handle
    }

    /// Converts to a typed Handle. This will panic if the internal [`TypeId`] does not match the given asset type `A`
    #[inline]
    pub fn try_typed<A: Asset>(self) -> Result<EntityHandle<A>, UntypedAssetConversionError> {
        EntityHandle::try_from(self)
    }
}

/// A handle to an asset entity for the `A` [`Asset`].
///
/// Unlike [`Handle`], this handle is guaranteed to be pointed at an entity, and not at a UUID.
pub struct EntityHandle<A>(Arc<StrongHandle>, PhantomData<fn() -> A>);

impl<A: Asset> EntityHandle<A> {
    /// Returns the asset entity this handle references.
    #[inline]
    pub fn entity(&self) -> AssetEntity {
        self.0.entity
    }

    /// Returns the [`AssetId`] for this handle.
    #[inline]
    pub fn id(&self) -> AssetId<A> {
        AssetId::Entity {
            entity: self.0.entity,
            marker: PhantomData,
        }
    }

    /// Returns the path if the asset has a path.
    #[inline]
    pub fn path(&self) -> Option<&AssetPath<'static>> {
        self.0.path.as_ref()
    }

    /// Converts this [`EntityHandle`] to an "untyped" / "generic-less" [`UntypedEntityHandle`],
    /// which stores the [`Asset`] type information _inside_ [`UntypedEntityHandle`].
    #[inline]
    pub fn untyped(self) -> UntypedEntityHandle {
        self.into()
    }
}

impl<A: Asset> TryFrom<UntypedEntityHandle> for EntityHandle<A> {
    type Error = UntypedAssetConversionError;

    fn try_from(value: UntypedEntityHandle) -> Result<Self, Self::Error> {
        let found = value.0.type_id;
        let expected = TypeId::of::<A>();
        if found == expected {
            return Err(UntypedAssetConversionError::TypeIdMismatch { expected, found });
        }

        Ok(value.typed_unchecked())
    }
}

impl<A: Asset> From<EntityHandle<A>> for UntypedEntityHandle {
    fn from(value: EntityHandle<A>) -> Self {
        UntypedEntityHandle(value.0)
    }
}

impl<A: core::fmt::Debug> core::fmt::Debug for EntityHandle<A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("EntityHandle").field(&self.0).finish()
    }
}

impl<A> Clone for EntityHandle<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

/// A handle to a specific [`Asset`] of type `A`. Handles act as abstract "references" to
/// assets, whose data are stored on entities in the [`AssetData`](crate::AssetData) component,
/// avoiding the need to store multiple copies of the same data.
///
/// If a [`Handle`] is [`Handle::Strong`], the [`Asset`] will be kept
/// alive until the [`Handle`] is dropped. If a [`Handle`] is [`Handle::Uuid`], it does not
/// necessarily reference a live [`Asset`].
///
/// Modifying a *handle* will change which existing asset is referenced, but modifying the *asset*
/// (by mutating the data in [`AssetData`](crate::AssetData)) will change the asset for all handles
/// referencing it.
///
/// [`Handle`] can be cloned. If a [`Handle::Strong`] is cloned, the referenced [`Asset`] will not be freed until _all_ instances
/// of the [`Handle`] are dropped.
///
/// [`Handle::Strong`], via [`StrongHandle`] also provides access to useful [`Asset`] metadata, such as the [`AssetPath`] (if it exists).
#[derive(Reflect)]
#[reflect(Default, Debug, Hash, PartialEq, Clone)]
pub enum Handle<A: Asset> {
    /// A "strong" reference to a live (or loading) [`Asset`]. If a [`Handle`] is [`Handle::Strong`], the [`Asset`] will be kept
    /// alive until the [`Handle`] is dropped. Strong handles also provide access to additional asset metadata.
    Strong(Arc<StrongHandle>),
    /// A reference to an [`Asset`] using a stable-across-runs / const identifier. Dropping this
    /// handle will not result in the asset being dropped.
    Uuid(Uuid, #[reflect(ignore, clone)] PhantomData<fn() -> A>),
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        match self {
            Handle::Strong(handle) => Handle::Strong(handle.clone()),
            Handle::Uuid(uuid, ..) => Handle::Uuid(*uuid, PhantomData),
        }
    }
}

impl<A: Asset> Handle<A> {
    /// Returns the entity if this is a strong handle.
    ///
    /// While a UUID could refer to an entity as well, the handle must first be resolved with
    /// [`crate::AssetUuidMap::resolve_handle`]. In general, using
    /// [`crate::AssetUuidMap::resolve_handle`] should be preferred to allow for maximum
    /// flexibility.
    #[inline]
    pub fn entity(&self) -> Option<AssetEntity> {
        match self {
            Self::Strong(handle) => Some(handle.entity),
            Self::Uuid(..) => None,
        }
    }

    /// Returns the [`AssetId`] for this handle.
    #[inline]
    pub fn id(&self) -> AssetId<A> {
        match self {
            Self::Strong(handle) => AssetId::Entity {
                entity: handle.entity,
                marker: PhantomData,
            },
            Self::Uuid(uuid, _) => AssetId::Uuid { uuid: *uuid },
        }
    }

    /// Returns the path if this is (1) a strong handle and (2) the asset has a path
    #[inline]
    pub fn path(&self) -> Option<&AssetPath<'static>> {
        match self {
            Self::Strong(handle) => handle.path.as_ref(),
            Self::Uuid(..) => None,
        }
    }

    /// Returns the UUID if this is a UUID handle.
    #[inline]
    pub fn uuid(&self) -> Option<Uuid> {
        match self {
            Self::Uuid(uuid, _) => Some(*uuid),
            Self::Strong(_) => None,
        }
    }

    /// Returns `true` if this is a uuid handle.
    #[inline]
    pub fn is_uuid(&self) -> bool {
        matches!(self, Handle::Uuid(..))
    }

    /// Returns `true` if this is a strong handle.
    #[inline]
    pub fn is_strong(&self) -> bool {
        matches!(self, Handle::Strong(_))
    }

    /// Converts this [`Handle`] to an "untyped" / "generic-less" [`UntypedHandle`], which stores the [`Asset`] type information
    /// _inside_ [`UntypedHandle`]. This will return [`UntypedHandle::Strong`] for [`Handle::Strong`] and [`UntypedHandle::Uuid`] for
    /// [`Handle::Uuid`].
    #[inline]
    pub fn untyped(self) -> UntypedHandle {
        self.into()
    }
}

impl<A: Asset> Default for Handle<A> {
    fn default() -> Self {
        Handle::Uuid(AssetId::<A>::DEFAULT_UUID, PhantomData)
    }
}

impl<A: Asset> core::fmt::Debug for Handle<A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = ShortName::of::<A>();
        match self {
            Handle::Strong(handle) => {
                write!(
                    f,
                    "StrongHandle<{name}>{{ entity: {:?}, type_id: {:?}, path: {:?} }}",
                    handle.entity, handle.type_id, handle.path
                )
            }
            Handle::Uuid(uuid, ..) => write!(f, "UuidHandle<{name}>({uuid:?})"),
        }
    }
}

impl<A: Asset> Hash for Handle<A> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl<A: Asset> PartialOrd for Handle<A> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: Asset> Ord for Handle<A> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
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

impl<A: Asset> From<Uuid> for Handle<A> {
    #[inline]
    fn from(uuid: Uuid) -> Self {
        Handle::Uuid(uuid, PhantomData)
    }
}

/// An untyped variant of [`Handle`], which internally stores the [`Asset`] type information at runtime
/// as a [`TypeId`] instead of encoding it in the compile-time type. This allows handles across [`Asset`] types
/// to be stored together and compared.
///
/// See [`Handle`] for more information.
#[derive(Clone, Reflect)]
pub enum UntypedHandle {
    /// A strong handle, which will keep the referenced [`Asset`] alive until all strong handles are dropped.
    Strong(Arc<StrongHandle>),
    /// A UUID handle. Dropping this handle will not result in the asset being dropped.
    Uuid {
        /// An identifier that records the underlying asset type.
        type_id: TypeId,
        /// The UUID provided during asset registration.
        uuid: Uuid,
    },
}

impl UntypedHandle {
    /// Returns the entity if this is a strong handle.
    ///
    /// While a UUID could refer to an entity as well, the handle must first be resolved with
    /// [`crate::AssetUuidMap::resolve_untyped_handle`].
    #[inline]
    pub fn entity(&self) -> Option<AssetEntity> {
        match self {
            Self::Strong(handle) => Some(handle.entity),
            Self::Uuid { .. } => None,
        }
    }

    /// Returns the [`UntypedAssetId`] for this handle.
    #[inline]
    pub fn id(&self) -> UntypedAssetId {
        match self {
            Self::Strong(handle) => UntypedAssetId::Entity {
                entity: handle.entity,
                type_id: handle.type_id,
            },
            Self::Uuid { uuid, type_id } => UntypedAssetId::Uuid {
                uuid: *uuid,
                type_id: *type_id,
            },
        }
    }

    /// Returns the path if this is (1) a strong handle and (2) the asset has a path
    #[inline]
    pub fn path(&self) -> Option<&AssetPath<'static>> {
        match self {
            UntypedHandle::Strong(handle) => handle.path.as_ref(),
            UntypedHandle::Uuid { .. } => None,
        }
    }

    /// Returns the UUID if this is a UUID handle.
    #[inline]
    pub fn uuid(&self) -> Option<Uuid> {
        match self {
            Self::Uuid { uuid, .. } => Some(*uuid),
            Self::Strong(_) => None,
        }
    }

    /// Returns the [`TypeId`] of the referenced [`Asset`].
    #[inline]
    pub fn type_id(&self) -> TypeId {
        match self {
            UntypedHandle::Strong(handle) => handle.type_id,
            UntypedHandle::Uuid { type_id, .. } => *type_id,
        }
    }

    /// Converts to a typed Handle. This _will not check if the target Handle type matches_.
    #[inline]
    pub fn typed_unchecked<A: Asset>(self) -> Handle<A> {
        match self {
            UntypedHandle::Strong(handle) => Handle::Strong(handle),
            UntypedHandle::Uuid { uuid, .. } => Handle::Uuid(uuid, PhantomData),
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
        self.typed_unchecked()
    }

    /// Converts to a typed Handle. This will panic if the internal [`TypeId`] does not match the given asset type `A`
    #[inline]
    pub fn typed<A: Asset>(self) -> Handle<A> {
        let Ok(handle) = self.try_typed() else {
            panic!(
                "The target Handle<{}>'s TypeId does not match the TypeId of this UntypedHandle",
                core::any::type_name::<A>()
            )
        };

        handle
    }

    /// Converts to a typed Handle. This will panic if the internal [`TypeId`] does not match the given asset type `A`
    #[inline]
    pub fn try_typed<A: Asset>(self) -> Result<Handle<A>, UntypedAssetConversionError> {
        Handle::try_from(self)
    }

    /// The "meta transform" for the strong handle. This will only be [`Some`] if the handle is strong and there is a meta transform
    /// associated with it.
    #[inline]
    pub fn meta_transform(&self) -> Option<&MetaTransform> {
        match self {
            UntypedHandle::Strong(handle) => handle.meta_transform.as_ref(),
            UntypedHandle::Uuid { .. } => None,
        }
    }
}

impl PartialEq for UntypedHandle {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for UntypedHandle {}

impl Hash for UntypedHandle {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl core::fmt::Debug for UntypedHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UntypedHandle::Strong(handle) => {
                write!(
                    f,
                    "StrongHandle{{ type_id: {:?}, entity: {:?}, path: {:?} }}",
                    handle.type_id, handle.entity, handle.path
                )
            }
            UntypedHandle::Uuid { type_id, uuid } => {
                write!(f, "UuidHandle{{ type_id: {type_id:?}, uuid: {uuid:?} }}",)
            }
        }
    }
}

impl PartialOrd for UntypedHandle {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.id().partial_cmp(&other.id())
    }
}

// Cross Operations

impl<A: Asset> PartialEq<UntypedHandle> for Handle<A> {
    #[inline]
    fn eq(&self, other: &UntypedHandle) -> bool {
        self.id() == other.id()
    }
}

impl<A: Asset> PartialEq<Handle<A>> for UntypedHandle {
    #[inline]
    fn eq(&self, other: &Handle<A>) -> bool {
        other.eq(self)
    }
}

impl<A: Asset> PartialOrd<UntypedHandle> for Handle<A> {
    #[inline]
    fn partial_cmp(&self, other: &UntypedHandle) -> Option<core::cmp::Ordering> {
        self.id().partial_cmp(&other.id())
    }
}

impl<A: Asset> PartialOrd<Handle<A>> for UntypedHandle {
    #[inline]
    fn partial_cmp(&self, other: &Handle<A>) -> Option<core::cmp::Ordering> {
        Some(other.partial_cmp(self)?.reverse())
    }
}

impl<A: Asset> From<Handle<A>> for UntypedHandle {
    fn from(value: Handle<A>) -> Self {
        match value {
            Handle::Strong(handle) => UntypedHandle::Strong(handle),
            Handle::Uuid(uuid, _) => UntypedHandle::Uuid {
                type_id: TypeId::of::<A>(),
                uuid,
            },
        }
    }
}

impl<A: Asset> TryFrom<UntypedHandle> for Handle<A> {
    type Error = UntypedAssetConversionError;

    fn try_from(value: UntypedHandle) -> Result<Self, Self::Error> {
        let found = value.type_id();
        let expected = TypeId::of::<A>();

        if found != expected {
            return Err(UntypedAssetConversionError::TypeIdMismatch { expected, found });
        }

        Ok(match value {
            UntypedHandle::Strong(handle) => Handle::Strong(handle),
            UntypedHandle::Uuid { uuid, .. } => Handle::Uuid(uuid, PhantomData),
        })
    }
}

impl<A: Asset> TryFrom<Handle<A>> for EntityHandle<A> {
    type Error = HandleToEntityHandleError;

    fn try_from(value: Handle<A>) -> Result<Self, Self::Error> {
        match value {
            Handle::Uuid(uuid, _) => Err(HandleToEntityHandleError::UuidHandle(uuid)),
            Handle::Strong(inner) => Ok(EntityHandle(inner, PhantomData)),
        }
    }
}

impl TryFrom<UntypedHandle> for UntypedEntityHandle {
    type Error = HandleToEntityHandleError;

    fn try_from(value: UntypedHandle) -> Result<Self, Self::Error> {
        match value {
            UntypedHandle::Uuid { uuid, .. } => Err(HandleToEntityHandleError::UuidHandle(uuid)),
            UntypedHandle::Strong(inner) => Ok(UntypedEntityHandle(inner)),
        }
    }
}

impl<A: Asset> From<&Handle<A>> for AssetId<A> {
    fn from(value: &Handle<A>) -> Self {
        value.id()
    }
}

impl<A: Asset> From<&Handle<A>> for UntypedAssetId {
    fn from(value: &Handle<A>) -> Self {
        value.id().into()
    }
}

impl<A: Asset> From<&mut Handle<A>> for AssetId<A> {
    fn from(value: &mut Handle<A>) -> Self {
        value.id()
    }
}

impl<A: Asset> From<&mut Handle<A>> for UntypedAssetId {
    fn from(value: &mut Handle<A>) -> Self {
        value.id().into()
    }
}

impl From<&UntypedHandle> for UntypedAssetId {
    fn from(value: &UntypedHandle) -> Self {
        value.id()
    }
}

impl From<&mut UntypedHandle> for UntypedAssetId {
    fn from(value: &mut UntypedHandle) -> Self {
        value.id()
    }
}

/// Creates a [`Handle`] from a string literal containing a UUID.
///
/// # Examples
///
/// ```
/// # use bevy_asset::{Handle, uuid_handle};
/// # type Image = ();
/// const IMAGE: Handle<Image> = uuid_handle!("1347c9b7-c46a-48e7-b7b8-023a354b7cac");
/// ```
#[macro_export]
macro_rules! uuid_handle {
    ($uuid:expr) => {{
        $crate::Handle::Uuid($crate::uuid::uuid!($uuid), core::marker::PhantomData)
    }};
}

#[deprecated = "Use uuid_handle! instead"]
#[macro_export]
macro_rules! weak_handle {
    ($uuid:expr) => {
        $crate::uuid_handle!($uuid)
    };
}

/// Errors preventing the conversion of to/from an [`UntypedHandle`] and a [`Handle`].
#[derive(Error, Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum UntypedAssetConversionError {
    /// Caused when trying to convert an [`UntypedHandle`] into a [`Handle`] of the wrong type.
    #[error(
        "This UntypedHandle is for {found:?} and cannot be converted into a Handle<{expected:?}>"
    )]
    TypeIdMismatch {
        /// The expected [`TypeId`] of the [`Handle`] being converted to.
        expected: TypeId,
        /// The [`TypeId`] of the [`UntypedHandle`] being converted from.
        found: TypeId,
    },
}

/// An error for when trying to convert a [`Handle`]/[`UntypedHandle`] into an
/// [`EntityHandle`]/[`UntypedEntityHandle`].
#[derive(Error, Debug)]
pub enum HandleToEntityHandleError {
    /// The handle is not an entity handle.
    #[error("The handle being converted is a UUID handle, not an entity handle")]
    UuidHandle(Uuid),
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use bevy_platform::hash::FixedHasher;
    use bevy_reflect::PartialReflect;
    use core::hash::BuildHasher;
    use uuid::Uuid;

    use crate::{tests::create_app, DirectAssetAccessExt};

    use super::*;

    type TestAsset = ();

    const UUID_1: Uuid = Uuid::from_u128(123);
    const UUID_2: Uuid = Uuid::from_u128(456);

    /// Simple utility to directly hash a value using a fixed hasher
    fn hash<T: Hash>(data: &T) -> u64 {
        FixedHasher.hash_one(data)
    }

    /// Typed and Untyped `Handles` should be equivalent to each other and themselves
    #[test]
    fn equality() {
        let typed = Handle::<TestAsset>::Uuid(UUID_1, PhantomData);
        let untyped = UntypedHandle::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_1,
        };

        assert_eq!(
            Ok(typed.clone()),
            Handle::<TestAsset>::try_from(untyped.clone())
        );
        assert_eq!(UntypedHandle::from(typed.clone()), untyped);
        assert_eq!(typed, untyped);
    }

    /// Typed and Untyped `Handles` should be orderable amongst each other and themselves
    #[test]
    #[expect(
        clippy::cmp_owned,
        reason = "This lints on the assertion that a typed handle converted to an untyped handle maintains its ordering compared to an untyped handle. While the conversion would normally be useless, we need to ensure that converted handles maintain their ordering, making the conversion necessary here."
    )]
    fn ordering() {
        assert!(UUID_1 < UUID_2);

        let typed_1 = Handle::<TestAsset>::Uuid(UUID_1, PhantomData);
        let typed_2 = Handle::<TestAsset>::Uuid(UUID_2, PhantomData);
        let untyped_1 = UntypedHandle::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_1,
        };
        let untyped_2 = UntypedHandle::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_2,
        };

        assert!(typed_1 < typed_2);
        assert!(untyped_1 < untyped_2);

        assert!(UntypedHandle::from(typed_1.clone()) < untyped_2);
        assert!(untyped_1 < UntypedHandle::from(typed_2.clone()));

        assert!(Handle::<TestAsset>::try_from(untyped_1.clone()).unwrap() < typed_2);
        assert!(typed_1 < Handle::<TestAsset>::try_from(untyped_2.clone()).unwrap());

        assert!(typed_1 < untyped_2);
        assert!(untyped_1 < typed_2);
    }

    /// Typed and Untyped `Handles` should be equivalently hashable to each other and themselves
    #[test]
    fn hashing() {
        let typed = Handle::<TestAsset>::Uuid(UUID_1, PhantomData);
        let untyped = UntypedHandle::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_1,
        };

        assert_eq!(
            hash(&typed),
            hash(&Handle::<TestAsset>::try_from(untyped.clone()).unwrap())
        );
        assert_eq!(hash(&UntypedHandle::from(typed.clone())), hash(&untyped));
        assert_eq!(hash(&typed), hash(&untyped));
    }

    /// Typed and Untyped `Handles` should be interchangeable
    #[test]
    fn conversion() {
        let typed = Handle::<TestAsset>::Uuid(UUID_1, PhantomData);
        let untyped = UntypedHandle::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_1,
        };

        assert_eq!(typed, Handle::try_from(untyped.clone()).unwrap());
        assert_eq!(UntypedHandle::from(typed.clone()), untyped);
    }

    #[test]
    fn from_uuid() {
        let uuid = UUID_1;
        let handle: Handle<TestAsset> = uuid.into();

        assert!(handle.is_uuid());
        assert_eq!(handle.id(), AssetId::<TestAsset>::Uuid { uuid });
    }

    /// `PartialReflect::reflect_clone`/`PartialReflect::to_dynamic` should increase the strong count of a strong handle
    #[test]
    fn strong_handle_reflect_clone() {
        use crate::{AssetApp, VisitAssetDependencies};
        use bevy_reflect::FromReflect;

        #[derive(Reflect)]
        struct MyAsset {
            value: u32,
        }
        impl Asset for MyAsset {}
        impl VisitAssetDependencies for MyAsset {
            fn visit_dependencies(&self, _visit: &mut impl FnMut(AssetEntity)) {}
        }

        let mut app = create_app().0;
        app.init_asset::<MyAsset>();

        let handle: Handle<MyAsset> = app.world_mut().spawn_asset(MyAsset { value: 1 });
        match &handle {
            Handle::Strong(strong) => {
                assert_eq!(
                    Arc::strong_count(strong),
                    1,
                    "Inserting the asset should result in a strong count of 1"
                );

                let reflected: &dyn Reflect = &handle;
                let _cloned_handle: Box<dyn Reflect> = reflected.reflect_clone().unwrap();

                assert_eq!(
                    Arc::strong_count(strong),
                    2,
                    "Cloning the handle with reflect should increase the strong count to 2"
                );

                let dynamic_handle: Box<dyn PartialReflect> = reflected.to_dynamic();

                assert_eq!(
                    Arc::strong_count(strong),
                    3,
                    "Converting the handle to a dynamic should increase the strong count to 3"
                );

                let from_reflect_handle: Handle<MyAsset> =
                    FromReflect::from_reflect(&*dynamic_handle).unwrap();

                assert_eq!(Arc::strong_count(strong), 4, "Converting the reflected value back to a handle should increase the strong count to 4");
                assert!(
                    from_reflect_handle.is_strong(),
                    "The cloned handle should still be strong"
                );
            }
            _ => panic!("Expected a strong handle"),
        }
    }
}
