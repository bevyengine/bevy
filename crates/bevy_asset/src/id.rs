use crate::{Asset, AssetIndex, Handle, UntypedHandle};
use bevy_reflect::{Reflect, Uuid};
use serde::{Deserialize, Serialize};
use std::{
    any::TypeId,
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
};
#[derive(Reflect, Serialize, Deserialize)]
pub enum AssetId<A: Asset> {
    Index {
        index: AssetIndex,
        #[reflect(ignore)]
        #[serde(skip_serializing)]
        marker: PhantomData<fn() -> A>,
    },
    Uuid {
        // TODO: implement reflect for this or replace it with an unsigned int?
        #[reflect(ignore)]
        uuid: Uuid,
    },
}

impl<A: Asset> AssetId<A> {
    /// The uuid for the default [`AssetId`]. It is valid to assign a value to this in [`Assets`](crate::Assets)
    /// and by convention (where appropriate) assets should support this pattern.
    pub const DEFAULT_UUID: Uuid = Uuid::from_u128(200809721996911295814598172825939264631);

    /// This asset id _should_ never be valid. Assigning a value to this in [`Assets`](crate::Assets) will
    /// produce undefined behavior, so don't do it!
    pub const INVALID_UUID: Uuid = Uuid::from_u128(108428345662029828789348721013522787528);

    /// Returns an [`AssetId`] with [`Self::INVALID_UUID`], which _should_ never be valid.
    #[inline]
    pub const fn invalid() -> Self {
        Self::Uuid {
            uuid: Self::INVALID_UUID,
        }
    }

    #[inline]
    pub(crate) fn internal(self) -> InternalAssetId {
        match self {
            AssetId::Index { index, .. } => InternalAssetId::Index(index),
            AssetId::Uuid { uuid } => InternalAssetId::Uuid(uuid),
        }
    }

    #[inline]
    pub fn untyped(self) -> UntypedAssetId {
        match self {
            AssetId::Index { index, .. } => UntypedAssetId::Index {
                index,
                type_id: TypeId::of::<A>(),
            },
            AssetId::Uuid { uuid } => UntypedAssetId::Uuid {
                uuid,
                type_id: TypeId::of::<A>(),
            },
        }
    }
}

impl<A: Asset> Default for AssetId<A> {
    fn default() -> Self {
        AssetId::Uuid {
            uuid: Self::DEFAULT_UUID,
        }
    }
}

impl<A: Asset> Clone for AssetId<A> {
    fn clone(&self) -> Self {
        match self {
            Self::Index { index, .. } => Self::Index {
                index: *index,
                marker: PhantomData,
            },
            Self::Uuid { uuid } => Self::Uuid { uuid: *uuid },
        }
    }
}

impl<A: Asset> Copy for AssetId<A> {}

impl<A: Asset> Display for AssetId<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetId::Index { index, .. } => {
                write!(
                    f,
                    "AssetId<{}>{{ index: {}, generation: {}}}",
                    std::any::type_name::<A>(),
                    index.index,
                    index.generation
                )
            }
            AssetId::Uuid { uuid } => {
                write!(
                    f,
                    "AssetId<{}>{{uuid: {}}}",
                    std::any::type_name::<A>(),
                    uuid
                )
            }
        }
    }
}
impl<A: Asset> Debug for AssetId<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetId::Index { index, .. } => {
                write!(
                    f,
                    "AssetId<{}>{{ index: {}, generation: {}}}",
                    std::any::type_name::<A>(),
                    index.index,
                    index.generation
                )
            }
            AssetId::Uuid { uuid } => {
                write!(
                    f,
                    "AssetId<{}>{{uuid: {}}}",
                    std::any::type_name::<A>(),
                    uuid
                )
            }
        }
    }
}

impl<A: Asset> Hash for AssetId<A> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.internal().hash(state);
    }
}

impl<A: Asset> PartialEq for AssetId<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.internal().eq(&other.internal())
    }
}

impl<A: Asset> Eq for AssetId<A> {}

impl<A: Asset> PartialOrd for AssetId<A> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: Asset> Ord for AssetId<A> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self {
            AssetId::Index { index, .. } => match other {
                AssetId::Index {
                    index: other_index, ..
                } => index.cmp(other_index),
                AssetId::Uuid { .. } => std::cmp::Ordering::Less,
            },
            AssetId::Uuid { uuid } => match other {
                AssetId::Index { .. } => std::cmp::Ordering::Greater,
                AssetId::Uuid { uuid: other_uuid } => uuid.cmp(other_uuid),
            },
        }
    }
}

impl<A: Asset> From<AssetIndex> for AssetId<A> {
    #[inline]
    fn from(value: AssetIndex) -> Self {
        Self::Index {
            index: value,
            marker: PhantomData,
        }
    }
}

impl<A: Asset> From<Uuid> for AssetId<A> {
    #[inline]
    fn from(value: Uuid) -> Self {
        Self::Uuid { uuid: value }
    }
}

impl<A: Asset> From<Handle<A>> for AssetId<A> {
    #[inline]
    fn from(value: Handle<A>) -> Self {
        value.id()
    }
}

impl<A: Asset> From<&Handle<A>> for AssetId<A> {
    #[inline]
    fn from(value: &Handle<A>) -> Self {
        value.id()
    }
}

impl<A: Asset> From<UntypedHandle> for AssetId<A> {
    #[inline]
    fn from(value: UntypedHandle) -> Self {
        value.id().typed()
    }
}

impl<A: Asset> From<&UntypedHandle> for AssetId<A> {
    #[inline]
    fn from(value: &UntypedHandle) -> Self {
        value.id().typed()
    }
}

impl<A: Asset> From<UntypedAssetId> for AssetId<A> {
    #[inline]
    fn from(value: UntypedAssetId) -> Self {
        value.typed()
    }
}

impl<A: Asset> From<&UntypedAssetId> for AssetId<A> {
    #[inline]
    fn from(value: &UntypedAssetId) -> Self {
        value.typed()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum UntypedAssetId {
    Index { type_id: TypeId, index: AssetIndex },
    Uuid { type_id: TypeId, uuid: Uuid },
}

impl UntypedAssetId {
    #[inline]
    pub fn typed_unchecked<A: Asset>(self) -> AssetId<A> {
        match self {
            UntypedAssetId::Index { index, .. } => AssetId::Index {
                index,
                marker: PhantomData,
            },
            UntypedAssetId::Uuid { uuid, .. } => AssetId::Uuid { uuid },
        }
    }

    #[inline]
    pub fn typed_debug_checked<A: Asset>(self) -> AssetId<A> {
        debug_assert_eq!(
            self.type_id(),
            TypeId::of::<A>(),
            "The target AssetId<{}>'s TypeId does not match the TypeId of this UntypedAssetId",
            std::any::type_name::<A>()
        );
        self.typed_unchecked()
    }

    #[inline]
    pub fn typed<A: Asset>(self) -> AssetId<A> {
        assert_eq!(
            self.type_id(),
            TypeId::of::<A>(),
            "The target AssetId<{}>'s TypeId does not match the TypeId of this UntypedAssetId",
            std::any::type_name::<A>()
        );
        self.typed_unchecked()
    }

    #[inline]
    pub(crate) fn internal(self) -> InternalAssetId {
        match self {
            UntypedAssetId::Index { index, .. } => InternalAssetId::Index(index),
            UntypedAssetId::Uuid { uuid, .. } => InternalAssetId::Uuid(uuid),
        }
    }

    #[inline]
    pub fn type_id(&self) -> TypeId {
        match self {
            UntypedAssetId::Index { type_id, .. } | UntypedAssetId::Uuid { type_id, .. } => {
                *type_id
            }
        }
    }
}

impl Display for UntypedAssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UntypedAssetId::Index { index, type_id } => {
                write!(
                    f,
                    "UntypedAssetId{{ type_id: {:?} index: {}, generation: {}}}",
                    *type_id, index.index, index.generation
                )
            }
            UntypedAssetId::Uuid { uuid, type_id } => {
                write!(
                    f,
                    "UntypedAssetId{{ type_id: {:?} uuid: {}}}",
                    *type_id, uuid
                )
            }
        }
    }
}

impl<A: Asset> From<AssetId<A>> for UntypedAssetId {
    #[inline]
    fn from(value: AssetId<A>) -> Self {
        value.untyped()
    }
}

impl<A: Asset> From<Handle<A>> for UntypedAssetId {
    #[inline]
    fn from(value: Handle<A>) -> Self {
        value.id().untyped()
    }
}

impl<A: Asset> From<&Handle<A>> for UntypedAssetId {
    #[inline]
    fn from(value: &Handle<A>) -> Self {
        value.id().untyped()
    }
}

/// An asset id without static or dynamic types associated with it.
/// This exist to support efficient type erased id drop tracking. We
/// could use [`UntypedAssetId`] for this, but the [`TypeId`] is unnecessary.
///
/// Do not _ever_ use this across asset types for comparison.
/// [`InternalAssetId`] contains no type information and will happily collide
/// with indices across types.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) enum InternalAssetId {
    Index(AssetIndex),
    Uuid(Uuid),
}

impl InternalAssetId {
    #[inline]
    pub fn typed<A: Asset>(self) -> AssetId<A> {
        match self {
            InternalAssetId::Index(index) => AssetId::Index {
                index,
                marker: PhantomData,
            },
            InternalAssetId::Uuid(uuid) => AssetId::Uuid { uuid },
        }
    }

    #[inline]
    pub fn untyped(self, type_id: TypeId) -> UntypedAssetId {
        match self {
            InternalAssetId::Index(index) => UntypedAssetId::Index { index, type_id },
            InternalAssetId::Uuid(uuid) => UntypedAssetId::Uuid { uuid, type_id },
        }
    }
}

impl From<AssetIndex> for InternalAssetId {
    fn from(value: AssetIndex) -> Self {
        Self::Index(value)
    }
}

impl From<Uuid> for InternalAssetId {
    fn from(value: Uuid) -> Self {
        Self::Uuid(value)
    }
}
