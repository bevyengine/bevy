pub use bevy_reflect_derive::TypeUuid;
pub use bevy_utils::Uuid;

pub trait TypeUuid {
    const TYPE_UUID: Uuid;
}

pub trait TypeUuidDynamic {
    fn type_uuid(&self) -> Uuid;
}

impl<T> TypeUuidDynamic for T
where
    T: TypeUuid,
{
    fn type_uuid(&self) -> Uuid {
        Self::TYPE_UUID
    }
}
