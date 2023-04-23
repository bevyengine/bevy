mod de;
mod ser;
mod type_data;

pub use de::*;
pub use ser::*;
pub use type_data::*;

#[cfg(test)]
mod tests {
    use crate::{self as bevy_reflect, DynamicEnum, DynamicTuple, DynamicTupleStruct};
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
    fn test_serialization_enum() {
        #[derive(Debug, Reflect)]
        enum TestEnum {
            Unit,
            Tuple(
                i32,
                #[reflect(skip_serializing)] i32,
                #[reflect(ignore)] i32,
            ),
            Struct {
                a: i32,
                b: i32,
                #[reflect(skip_serializing)]
                c: i32,
                #[reflect(ignore)]
                #[allow(dead_code)]
                d: i32,
            },
        }

        let mut registry = TypeRegistry::default();
        registry.register::<TestEnum>();

        macro_rules! assert_enum {
            ($expected: ident, $input: ident) => {
                let serializer = ReflectSerializer::new(&$input, &registry);
                let serialized =
                    ron::ser::to_string_pretty(&serializer, Default::default()).unwrap();

                let mut deserializer = ron::de::Deserializer::from_str(&serialized).unwrap();
                let deserialized = UntypedReflectDeserializer::new(&registry)
                    .deserialize(&mut deserializer)
                    .unwrap()
                    .take::<DynamicEnum>()
                    .unwrap();

                assert!(
                    $expected.reflect_partial_eq(&deserialized).unwrap(),
                    "expected `{:?}`, but found `{:?}`",
                    $expected,
                    deserialized
                );
            };
        }

        let test_enum = TestEnum::Unit;
        let expected = DynamicEnum::from(TestEnum::Unit);
        assert_enum!(expected, test_enum);

        let test_enum = TestEnum::Tuple(1, 2, 3);
        let expected = DynamicEnum::new(Reflect::type_name(&test_enum), "Tuple", {
            let mut dyn_tuple = DynamicTuple::default();
            dyn_tuple.insert(1);
            dyn_tuple
        });
        assert_enum!(expected, test_enum);

        let test_enum = TestEnum::Struct {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
        };
        let expected = DynamicEnum::new(Reflect::type_name(&test_enum), "Struct", {
            let mut dyn_struct = DynamicStruct::default();
            dyn_struct.insert("a", 1);
            dyn_struct.insert("b", 2);
            dyn_struct
        });
        assert_enum!(expected, test_enum);
    }
}
