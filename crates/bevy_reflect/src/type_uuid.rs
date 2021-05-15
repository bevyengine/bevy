pub use bevy_reflect_derive::TypeUuid;
pub use bevy_utils::Uuid;

pub trait TypeUuid {
    const TYPE_UUID: Uuid;
}

pub trait TypeUuidDynamic {
    fn type_uuid(&self) -> Uuid;
    fn type_name(&self) -> &'static str;
}

impl<T> TypeUuidDynamic for T
where
    T: TypeUuid,
{
    fn type_uuid(&self) -> Uuid {
        Self::TYPE_UUID
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(TypeUuid)]
    #[uuid = "af6466c2-a9f4-11eb-bcbc-0242ac130002"]
    struct TestDeriveStruct<T>
    where
        T: Clone,
    {
        _value: T,
    }

    fn test_impl_type_uuid(_: &impl TypeUuid) {}

    #[test]
    fn test_generic_type_uuid_derive() {
        let test_struct = TestDeriveStruct { _value: 42 };
        test_impl_type_uuid(&test_struct);
    }
}
