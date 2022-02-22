pub use bevy_reflect_derive::TypeUuid;
pub use bevy_utils::Uuid;

pub type UniqueAssetId = u64;

/// A trait for types with a statically associated UUID.
pub trait TypeUuid {
    const TYPE_UUID: UniqueAssetId;
}

/// A trait for types with an associated UUID.
pub trait TypeUuidDynamic {
    fn type_uuid(&self) -> UniqueAssetId;
    fn type_name(&self) -> &'static str;
}

impl<T> TypeUuidDynamic for T
where
    T: TypeUuid,
{
    /// Returns the UUID associated with this value's type.
    fn type_uuid(&self) -> UniqueAssetId {
        Self::TYPE_UUID
    }

    /// Returns the [type name] of this value's type.
    ///
    /// [type name]: std::any::type_name
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
