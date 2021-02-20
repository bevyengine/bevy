pub use bevy_reflect_derive::TypeUuid;
pub use bevy_utils::Uuid;

pub trait TypeUuid {
    const TYPE_UUID: Uuid;
}

pub trait TypeUuidDynamic {
    fn type_uuid(&self) -> Uuid;
    /// Helper to display the type in a readable manner to the user
    fn display_type(&self) -> String;
}

impl<T> TypeUuidDynamic for T
where
    T: TypeUuid,
{
    fn type_uuid(&self) -> Uuid {
        Self::TYPE_UUID
    }

    fn display_type(&self) -> String {
        format!(
            "{:?} (UUID {:?})",
            std::any::type_name::<Self>(),
            self.type_uuid()
        )
    }
}
