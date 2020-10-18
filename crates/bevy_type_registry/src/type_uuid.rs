pub use bevy_derive::TypeUuid;
use uuid::Uuid;

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
