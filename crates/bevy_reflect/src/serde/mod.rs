mod de;
mod ser;
mod type_data;

pub use de::*;
pub use ser::*;
pub use type_data::*;

#[cfg(test)]
mod tests {
    use crate::{self as bevy_reflect, DynamicTupleStruct};
    use crate::{
        serde::{ReflectSerializer, UntypedReflectDeserializer},
        type_registry::TypeRegistry,
        DynamicStruct, Reflect,
    };
    use serde::de::DeserializeSeed;

    #[test]
    fn test_serialization_struct() {
        #[derive(Debug, Reflect, PartialEq)]
        #[reflect(PartialEq)]
        struct TestStruct {
            a: i32,
            #[reflect(ignore)]
            b: i32,
            #[reflect(skip_serializing)]
            c: i32,
            d: i32,
        }

        let mut registry = TypeRegistry::default();
        registry.register::<TestStruct>();

        let test_struct = TestStruct {
            a: 3,
            b: 4,
            c: 5,
            d: 6,
        };

        let serializer = ReflectSerializer::new(&test_struct, &registry);
        let serialized =
            ron::ser::to_string_pretty(&serializer, ron::ser::PrettyConfig::default()).unwrap();

        let mut expected = DynamicStruct::default();
        expected.insert("a", 3);
        expected.insert("d", 6);

        let mut deserializer = ron::de::Deserializer::from_str(&serialized).unwrap();
        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
        let value = reflect_deserializer.deserialize(&mut deserializer).unwrap();
        let deserialized = value.take::<DynamicStruct>().unwrap();

        assert!(
            expected.reflect_partial_eq(&deserialized).unwrap(),
            "Expected {expected:?} found {deserialized:?}"
        );
    }

    #[test]
    fn test_serialization_tuple_struct() {
        #[derive(Debug, Reflect, PartialEq)]
        #[reflect(PartialEq)]
        struct TestStruct(
            i32,
            #[reflect(ignore)] i32,
            #[reflect(skip_serializing)] i32,
            i32,
        );

        let mut registry = TypeRegistry::default();
        registry.register::<TestStruct>();

        let test_struct = TestStruct(3, 4, 5, 6);

        let serializer = ReflectSerializer::new(&test_struct, &registry);
        let serialized =
            ron::ser::to_string_pretty(&serializer, ron::ser::PrettyConfig::default()).unwrap();

        let mut expected = DynamicTupleStruct::default();
        expected.insert(3);
        expected.insert(6);

        let mut deserializer = ron::de::Deserializer::from_str(&serialized).unwrap();
        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
        let value = reflect_deserializer.deserialize(&mut deserializer).unwrap();
        let deserialized = value.take::<DynamicTupleStruct>().unwrap();

        assert!(
            expected.reflect_partial_eq(&deserialized).unwrap(),
            "Expected {expected:?} found {deserialized:?}"
        );
    }

    #[test]
    #[should_panic(expected = "cannot get type info for bevy_reflect::struct_trait::DynamicStruct")]
    fn unproxied_dynamic_should_not_serialize() {
        let registry = TypeRegistry::default();

        let mut value = DynamicStruct::default();
        value.insert("foo", 123_u32);

        let serializer = ReflectSerializer::new(&value, &registry);
        ron::ser::to_string(&serializer).unwrap();
    }
}
