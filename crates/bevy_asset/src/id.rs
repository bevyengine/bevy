use crate::{Asset, AssetIndex};
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::{
    any::TypeId,
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
};
use thiserror::Error;

/// A unique runtime-only identifier for an [`Asset`]. This is cheap to [`Copy`]/[`Clone`] and is not directly tied to the
/// lifetime of the Asset. This means it _can_ point to an [`Asset`] that no longer exists.
///
/// For an identifier tied to the lifetime of an asset, see [`Handle`](`crate::Handle`).
///
/// For an "untyped" / "generic-less" id, see [`UntypedAssetId`].
#[derive(Reflect, Serialize, Deserialize)]
pub enum AssetId<A: Asset> {
    /// A small / efficient runtime identifier that can be used to efficiently look up an asset stored in [`Assets`]. This is
    /// the "default" identifier used for assets. The alternative(s) (ex: [`AssetId::Uuid`]) will only be used if assets are
    /// explicitly registered that way.
    ///
    /// [`Assets`]: crate::Assets
    Index {
        index: AssetIndex,
        #[reflect(ignore)]
        marker: PhantomData<fn() -> A>,
    },
    /// A stable-across-runs / const asset identifier. This will only be used if an asset is explicitly registered in [`Assets`]
    /// with one.
    ///
    /// [`Assets`]: crate::Assets
    Uuid { uuid: Uuid },
}

impl<A: Asset> AssetId<A> {
    /// The uuid for the default [`AssetId`]. It is valid to assign a value to this in [`Assets`](crate::Assets)
    /// and by convention (where appropriate) assets should support this pattern.
    pub const DEFAULT_UUID: Uuid = Uuid::from_u128(200809721996911295814598172825939264631);

    /// This asset id _should_ never be valid. Assigning a value to this in [`Assets`](crate::Assets) will
    /// produce undefined behavior, so don't do it!
    pub const INVALID_UUID: Uuid = Uuid::from_u128(108428345662029828789348721013522787528);

    /// Returns an [`AssetId`] with [`Self::INVALID_UUID`], which _should_ never be assigned to.
    #[inline]
    pub const fn invalid() -> Self {
        Self::Uuid {
            uuid: Self::INVALID_UUID,
        }
    }

    /// Converts this to an "untyped" / "generic-less" [`Asset`] identifier that stores the type information
    /// _inside_ the [`UntypedAssetId`].
    #[inline]
    pub fn untyped(self) -> UntypedAssetId {
        self.into()
    }

    #[inline]
    pub(crate) fn internal(self) -> InternalAssetId {
        match self {
            AssetId::Index { index, .. } => InternalAssetId::Index(index),
            AssetId::Uuid { uuid } => InternalAssetId::Uuid(uuid),
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
        *self
    }
}

impl<A: Asset> Copy for AssetId<A> {}

impl<A: Asset> Display for AssetId<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
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
        TypeId::of::<A>().hash(state);
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
        self.internal().cmp(&other.internal())
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

/// An "untyped" / "generic-less" [`Asset`] identifier that behaves much like [`AssetId`], but stores the [`Asset`] type
/// information at runtime instead of compile-time. This increases the size of the type, but it enables storing asset ids
/// across asset types together and enables comparisons between them.
#[derive(Debug, Copy, Clone)]
pub enum UntypedAssetId {
    /// A small / efficient runtime identifier that can be used to efficiently look up an asset stored in [`Assets`]. This is
    /// the "default" identifier used for assets. The alternative(s) (ex: [`UntypedAssetId::Uuid`]) will only be used if assets are
    /// explicitly registered that way.
    ///
    /// [`Assets`]: crate::Assets
    Index { type_id: TypeId, index: AssetIndex },
    /// A stable-across-runs / const asset identifier. This will only be used if an asset is explicitly registered in [`Assets`]
    /// with one.
    ///
    /// [`Assets`]: crate::Assets
    Uuid { type_id: TypeId, uuid: Uuid },
}

impl UntypedAssetId {
    /// Converts this to a "typed" [`AssetId`] without checking the stored type to see if it matches the target `A` [`Asset`] type.
    /// This should only be called if you are _absolutely certain_ the asset type matches the stored type. And even then, you should
    /// consider using [`UntypedAssetId::typed_debug_checked`] instead.
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

    /// Converts this to a "typed" [`AssetId`]. When compiled in debug-mode it will check to see if the stored type
    /// matches the target `A` [`Asset`] type. When compiled in release-mode, this check will be skipped.
    ///
    /// # Panics
    ///
    /// Panics if compiled in debug mode and the [`TypeId`] of `A` does not match the stored [`TypeId`].
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

    /// Converts this to a "typed" [`AssetId`].
    ///
    /// # Panics
    ///
    /// Panics if the [`TypeId`] of `A` does not match the stored type id.
    #[inline]
    pub fn typed<A: Asset>(self) -> AssetId<A> {
        let Ok(id) = self.try_typed() else {
            panic!(
                "The target AssetId<{}>'s TypeId does not match the TypeId of this UntypedAssetId",
                std::any::type_name::<A>()
            )
        };

        id
    }

    /// Try to convert this to a "typed" [`AssetId`].
    #[inline]
    pub fn try_typed<A: Asset>(self) -> Result<AssetId<A>, UntypedAssetIdConversionError> {
        AssetId::try_from(self)
    }

    /// Returns the stored [`TypeId`] of the referenced [`Asset`].
    #[inline]
    pub fn type_id(&self) -> TypeId {
        match self {
            UntypedAssetId::Index { type_id, .. } | UntypedAssetId::Uuid { type_id, .. } => {
                *type_id
            }
        }
    }

    #[inline]
    pub(crate) fn internal(self) -> InternalAssetId {
        match self {
            UntypedAssetId::Index { index, .. } => InternalAssetId::Index(index),
            UntypedAssetId::Uuid { uuid, .. } => InternalAssetId::Uuid(uuid),
        }
    }
}

impl Display for UntypedAssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut writer = f.debug_struct("UntypedAssetId");
        match self {
            UntypedAssetId::Index { index, type_id } => {
                writer
                    .field("type_id", type_id)
                    .field("index", &index.index)
                    .field("generation", &index.generation);
            }
            UntypedAssetId::Uuid { uuid, type_id } => {
                writer.field("type_id", type_id).field("uuid", uuid);
            }
        }
        writer.finish()
    }
}

impl PartialEq for UntypedAssetId {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.type_id() == other.type_id() && self.internal().eq(&other.internal())
    }
}

impl Eq for UntypedAssetId {}

impl Hash for UntypedAssetId {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.internal().hash(state);
        self.type_id().hash(state);
    }
}

impl PartialOrd for UntypedAssetId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.type_id() != other.type_id() {
            None
        } else {
            Some(self.internal().cmp(&other.internal()))
        }
    }
}

/// An asset id without static or dynamic types associated with it.
/// This exist to support efficient type erased id drop tracking. We
/// could use [`UntypedAssetId`] for this, but the [`TypeId`] is unnecessary.
///
/// Do not _ever_ use this across asset types for comparison.
/// [`InternalAssetId`] contains no type information and will happily collide
/// with indices across types.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub(crate) enum InternalAssetId {
    Index(AssetIndex),
    Uuid(Uuid),
}

impl InternalAssetId {
    #[inline]
    pub(crate) fn typed<A: Asset>(self) -> AssetId<A> {
        match self {
            InternalAssetId::Index(index) => AssetId::Index {
                index,
                marker: PhantomData,
            },
            InternalAssetId::Uuid(uuid) => AssetId::Uuid { uuid },
        }
    }

    #[inline]
    pub(crate) fn untyped(self, type_id: TypeId) -> UntypedAssetId {
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

// Cross Operations

impl<A: Asset> PartialEq<UntypedAssetId> for AssetId<A> {
    #[inline]
    fn eq(&self, other: &UntypedAssetId) -> bool {
        TypeId::of::<A>() == other.type_id() && self.internal().eq(&other.internal())
    }
}

impl<A: Asset> PartialEq<AssetId<A>> for UntypedAssetId {
    #[inline]
    fn eq(&self, other: &AssetId<A>) -> bool {
        other.eq(self)
    }
}

impl<A: Asset> PartialOrd<UntypedAssetId> for AssetId<A> {
    #[inline]
    fn partial_cmp(&self, other: &UntypedAssetId) -> Option<std::cmp::Ordering> {
        if TypeId::of::<A>() != other.type_id() {
            None
        } else {
            Some(self.internal().cmp(&other.internal()))
        }
    }
}

impl<A: Asset> PartialOrd<AssetId<A>> for UntypedAssetId {
    #[inline]
    fn partial_cmp(&self, other: &AssetId<A>) -> Option<std::cmp::Ordering> {
        Some(other.partial_cmp(self)?.reverse())
    }
}

impl<A: Asset> From<AssetId<A>> for UntypedAssetId {
    #[inline]
    fn from(value: AssetId<A>) -> Self {
        let type_id = TypeId::of::<A>();

        match value {
            AssetId::Index { index, .. } => UntypedAssetId::Index { type_id, index },
            AssetId::Uuid { uuid } => UntypedAssetId::Uuid { type_id, uuid },
        }
    }
}

impl<A: Asset> TryFrom<UntypedAssetId> for AssetId<A> {
    type Error = UntypedAssetIdConversionError;

    #[inline]
    fn try_from(value: UntypedAssetId) -> Result<Self, Self::Error> {
        let found = value.type_id();
        let expected = TypeId::of::<A>();

        match value {
            UntypedAssetId::Index { index, type_id } if type_id == expected => Ok(AssetId::Index {
                index,
                marker: PhantomData,
            }),
            UntypedAssetId::Uuid { uuid, type_id } if type_id == expected => {
                Ok(AssetId::Uuid { uuid })
            }
            _ => Err(UntypedAssetIdConversionError::TypeIdMismatch { expected, found }),
        }
    }
}

/// Errors preventing the conversion of to/from an [`UntypedAssetId`] and an [`AssetId`].
#[derive(Error, Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum UntypedAssetIdConversionError {
    /// Caused when trying to convert an [`UntypedAssetId`] into an [`AssetId`] of the wrong type.
    #[error("This UntypedAssetId is for {found:?} and cannot be converted into an AssetId<{expected:?}>")]
    TypeIdMismatch { expected: TypeId, found: TypeId },
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestAsset = ();

    const UUID_1: Uuid = Uuid::from_u128(123);
    const UUID_2: Uuid = Uuid::from_u128(456);

    /// Simple utility to directly hash a value using a fixed hasher
    fn hash<T: Hash>(data: &T) -> u64 {
        use std::hash::Hasher;

        let mut hasher = bevy_utils::AHasher::default();
        data.hash(&mut hasher);
        hasher.finish()
    }

    /// Typed and Untyped `AssetIds` should be equivalent to each other and themselves
    #[test]
    fn equality() {
        let typed = AssetId::<TestAsset>::Uuid { uuid: UUID_1 };
        let untyped = UntypedAssetId::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_1,
        };

        assert_eq!(Ok(typed), AssetId::try_from(untyped));
        assert_eq!(UntypedAssetId::from(typed), untyped);
        assert_eq!(typed, untyped);
    }

    /// Typed and Untyped `AssetIds` should be orderable amongst each other and themselves
    #[test]
    fn ordering() {
        assert!(UUID_1 < UUID_2);

        let typed_1 = AssetId::<TestAsset>::Uuid { uuid: UUID_1 };
        let typed_2 = AssetId::<TestAsset>::Uuid { uuid: UUID_2 };
        let untyped_1 = UntypedAssetId::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_1,
        };
        let untyped_2 = UntypedAssetId::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_2,
        };

        assert!(typed_1 < typed_2);
        assert!(untyped_1 < untyped_2);

        assert!(UntypedAssetId::from(typed_1) < untyped_2);
        assert!(untyped_1 < UntypedAssetId::from(typed_2));

        assert!(AssetId::try_from(untyped_1).unwrap() < typed_2);
        assert!(typed_1 < AssetId::try_from(untyped_2).unwrap());

        assert!(typed_1 < untyped_2);
        assert!(untyped_1 < typed_2);
    }

    /// Typed and Untyped `AssetIds` should be equivalently hashable to each other and themselves
    #[test]
    fn hashing() {
        let typed = AssetId::<TestAsset>::Uuid { uuid: UUID_1 };
        let untyped = UntypedAssetId::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_1,
        };

        assert_eq!(
            hash(&typed),
            hash(&AssetId::<TestAsset>::try_from(untyped).unwrap())
        );
        assert_eq!(hash(&UntypedAssetId::from(typed)), hash(&untyped));
        assert_eq!(hash(&typed), hash(&untyped));
    }

    /// Typed and Untyped `AssetIds` should be interchangeable
    #[test]
    fn conversion() {
        let typed = AssetId::<TestAsset>::Uuid { uuid: UUID_1 };
        let untyped = UntypedAssetId::Uuid {
            type_id: TypeId::of::<TestAsset>(),
            uuid: UUID_1,
        };

        assert_eq!(Ok(typed), AssetId::try_from(untyped));
        assert_eq!(UntypedAssetId::from(typed), untyped);
    }
}
