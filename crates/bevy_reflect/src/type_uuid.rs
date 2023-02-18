pub use bevy_reflect_derive::TypeUuid;
pub use bevy_utils::Uuid;

/// A trait for types with a statically associated UUID.
pub trait TypeUuid {
    const TYPE_UUID: Uuid;
}

/// A trait for types with an associated UUID.
pub trait TypeUuidDynamic {
    fn type_uuid(&self) -> Uuid;
    fn type_name(&self) -> &'static str;
}

impl<T> TypeUuidDynamic for T
where
    T: TypeUuid,
{
    /// Returns the UUID associated with this value's type.
    fn type_uuid(&self) -> Uuid {
        Self::TYPE_UUID
    }

    /// Returns the [type name] of this value's type.
    ///
    /// [type name]: std::any::type_name
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate as bevy_reflect;
    use bevy_reflect_derive::TypeUuid;
    use std::marker::PhantomData;

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
        #[derive(TypeUuid, Clone)]
        #[uuid = "ebb16cc9-4d5a-453c-aa8c-c72bd8ec83a2"]
        struct T;

        let test_struct = TestDeriveStruct { _value: T };
        test_impl_type_uuid(&test_struct);
    }

    #[test]
    fn test_generic_type_unique_uuid() {
        #[derive(TypeUuid, Clone)]
        #[uuid = "49951b1c-4811-45e7-acc6-3119249fbd8f"]
        struct A;

        #[derive(TypeUuid, Clone)]
        #[uuid = "4882b8f5-5556-4cee-bea6-a2e5991997b7"]
        struct B;

        let uuid_a = TestDeriveStruct::<A>::TYPE_UUID;
        let uuid_b = TestDeriveStruct::<B>::TYPE_UUID;

        assert_ne!(uuid_a, uuid_b);
        assert_ne!(uuid_a, A::TYPE_UUID);
        assert_ne!(uuid_b, B::TYPE_UUID);
    }

    #[test]
    fn test_inverted_generic_type_unique_uuid() {
        #[derive(TypeUuid, Clone)]
        #[uuid = "49951b1c-4811-45e7-acc6-3119249fbd8f"]
        struct Inner;

        #[derive(TypeUuid, Clone)]
        #[uuid = "23ebc0c3-ef69-4ea0-8c2a-dca1b4e27c0d"]
        struct TestDeriveStructA<T>
        where
            T: Clone,
        {
            _phantom: PhantomData<T>,
        }

        #[derive(TypeUuid, Clone)]
        #[uuid = "a82f9936-70cb-482a-bd3d-cb99d87de55f"]
        struct TestDeriveStructB<T>
        where
            T: Clone,
        {
            _phantom: PhantomData<T>,
        }

        let uuid_ab = TestDeriveStructA::<TestDeriveStructB<Inner>>::TYPE_UUID;
        let uuid_ba = TestDeriveStructB::<TestDeriveStructA<Inner>>::TYPE_UUID;

        assert_ne!(uuid_ab, uuid_ba);
        assert_ne!(uuid_ab, TestDeriveStructA::<Inner>::TYPE_UUID);
        assert_ne!(uuid_ba, TestDeriveStructB::<Inner>::TYPE_UUID);
    }

    #[test]
    fn test_generic_type_uuid_same_for_eq_param() {
        #[derive(TypeUuid, Clone)]
        #[uuid = "49951b1c-4811-45e7-acc6-3119249fbd8f"]
        struct A;

        #[derive(TypeUuid, Clone)]
        #[uuid = "49951b1c-4811-45e7-acc6-3119249fbd8f"]
        struct BButSameAsA;

        let uuid_a = TestDeriveStruct::<A>::TYPE_UUID;
        let uuid_b = TestDeriveStruct::<BButSameAsA>::TYPE_UUID;

        assert_eq!(uuid_a, uuid_b);
    }

    #[test]
    fn test_multiple_generic_uuid() {
        #[derive(TypeUuid)]
        #[uuid = "35c8a7d3-d4b3-4bd7-b847-1118dc78092f"]
        struct TestGeneric<A, B> {
            _value_a: A,
            _value_b: B,
        }
        assert_ne!(
            TestGeneric::<f32, bool>::TYPE_UUID,
            TestGeneric::<bool, f32>::TYPE_UUID
        );
    }

    #[test]
    fn test_primitive_generic_uuid() {
        test_impl_type_uuid(&true);
        test_impl_type_uuid(&Some(true));
        test_impl_type_uuid(&TestDeriveStruct::<bool> { _value: true });

        assert_ne!(Option::<bool>::TYPE_UUID, Option::<f32>::TYPE_UUID);

        assert_ne!(<[bool; 0]>::TYPE_UUID, <[bool; 1]>::TYPE_UUID);
        assert_ne!(<[bool; 0]>::TYPE_UUID, <[f32; 0]>::TYPE_UUID);

        assert_ne!(
            <(bool, bool)>::TYPE_UUID,
            <(bool, bool, bool, bool)>::TYPE_UUID
        );
        assert_ne!(<(bool, f32)>::TYPE_UUID, <(f32, bool)>::TYPE_UUID);
    }
}
