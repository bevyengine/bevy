pub use deserialize_with_registry::*;
pub use deserializer::*;
pub use processor::*;
pub use registrations::*;

mod arrays;
mod deserialize_with_registry;
mod deserializer;
mod enums;
mod error_utils;
mod helpers;
mod lists;
mod maps;
mod options;
mod processor;
mod registration_utils;
mod registrations;
mod sets;
mod struct_utils;
mod structs;
mod tuple_structs;
mod tuple_utils;
mod tuples;

#[cfg(test)]
mod tests {
    use alloc::{
        boxed::Box,
        string::{String, ToString},
        vec,
        vec::Vec,
    };
    use core::{any::TypeId, f32::consts::PI, ops::RangeInclusive};
    use serde::{de::DeserializeSeed, Deserialize};
    use serde::{de::IgnoredAny, Deserializer};

    use bevy_platform::collections::{HashMap, HashSet};

    use crate::{
        serde::{
            ReflectDeserializer, ReflectDeserializerProcessor, ReflectSerializer,
            TypedReflectDeserializer,
        },
        DynamicEnum, FromReflect, PartialReflect, Reflect, ReflectDeserialize, TypeRegistration,
        TypeRegistry,
    };

    #[derive(Reflect, Debug, PartialEq)]
    struct MyStruct {
        primitive_value: i8,
        option_value: Option<String>,
        option_value_complex: Option<SomeStruct>,
        tuple_value: (f32, usize),
        list_value: Vec<i32>,
        array_value: [i32; 5],
        map_value: HashMap<u8, usize>,
        set_value: HashSet<u8>,
        struct_value: SomeStruct,
        tuple_struct_value: SomeTupleStruct,
        unit_struct: SomeUnitStruct,
        unit_enum: SomeEnum,
        newtype_enum: SomeEnum,
        tuple_enum: SomeEnum,
        struct_enum: SomeEnum,
        ignored_struct: SomeIgnoredStruct,
        ignored_tuple_struct: SomeIgnoredTupleStruct,
        ignored_struct_variant: SomeIgnoredEnum,
        ignored_tuple_variant: SomeIgnoredEnum,
        custom_deserialize: CustomDeserialize,
    }

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeStruct {
        foo: i64,
    }

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeTupleStruct(String);

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeUnitStruct;

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeIgnoredStruct {
        #[reflect(ignore)]
        ignored: i32,
    }

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeIgnoredTupleStruct(#[reflect(ignore)] i32);

    #[derive(Reflect, Debug, PartialEq, Deserialize)]
    struct SomeDeserializableStruct {
        foo: i64,
    }

    /// Implements a custom deserialize using `#[reflect(Deserialize)]`.
    ///
    /// For testing purposes, this is just the auto-generated one from deriving.
    #[derive(Reflect, Debug, PartialEq, Deserialize)]
    #[reflect(Deserialize)]
    struct CustomDeserialize {
        value: usize,
        #[serde(alias = "renamed")]
        inner_struct: SomeDeserializableStruct,
    }

    #[derive(Reflect, Debug, PartialEq)]
    enum SomeEnum {
        Unit,
        NewType(usize),
        Tuple(f32, f32),
        Struct { foo: String },
    }

    #[derive(Reflect, Debug, PartialEq)]
    enum SomeIgnoredEnum {
        Tuple(#[reflect(ignore)] f32, #[reflect(ignore)] f32),
        Struct {
            #[reflect(ignore)]
            foo: String,
        },
    }

    fn get_registry() -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        registry.register::<MyStruct>();
        registry.register::<SomeStruct>();
        registry.register::<SomeTupleStruct>();
        registry.register::<SomeUnitStruct>();
        registry.register::<SomeIgnoredStruct>();
        registry.register::<SomeIgnoredTupleStruct>();
        registry.register::<CustomDeserialize>();
        registry.register::<SomeDeserializableStruct>();
        registry.register::<SomeEnum>();
        registry.register::<SomeIgnoredEnum>();
        registry.register::<i8>();
        registry.register::<String>();
        registry.register::<i64>();
        registry.register::<f32>();
        registry.register::<usize>();
        registry.register::<i32>();
        registry.register::<u8>();
        registry.register::<(f32, usize)>();
        registry.register::<[i32; 5]>();
        registry.register::<Vec<i32>>();
        registry.register::<HashMap<u8, usize>>();
        registry.register::<HashSet<u8>>();
        registry.register::<Option<SomeStruct>>();
        registry.register::<Option<String>>();
        registry.register_type_data::<Option<String>, ReflectDeserialize>();
        registry
    }

    fn get_my_struct() -> MyStruct {
        let mut map = <HashMap<_, _>>::default();
        map.insert(64, 32);

        let mut set = <HashSet<_>>::default();
        set.insert(64);

        MyStruct {
            primitive_value: 123,
            option_value: Some(String::from("Hello world!")),
            option_value_complex: Some(SomeStruct { foo: 123 }),
            tuple_value: (PI, 1337),
            list_value: vec![-2, -1, 0, 1, 2],
            array_value: [-2, -1, 0, 1, 2],
            map_value: map,
            set_value: set,
            struct_value: SomeStruct { foo: 999999999 },
            tuple_struct_value: SomeTupleStruct(String::from("Tuple Struct")),
            unit_struct: SomeUnitStruct,
            unit_enum: SomeEnum::Unit,
            newtype_enum: SomeEnum::NewType(123),
            tuple_enum: SomeEnum::Tuple(1.23, 3.21),
            struct_enum: SomeEnum::Struct {
                foo: String::from("Struct variant value"),
            },
            ignored_struct: SomeIgnoredStruct { ignored: 0 },
            ignored_tuple_struct: SomeIgnoredTupleStruct(0),
            ignored_struct_variant: SomeIgnoredEnum::Struct {
                foo: String::default(),
            },
            ignored_tuple_variant: SomeIgnoredEnum::Tuple(0.0, 0.0),
            custom_deserialize: CustomDeserialize {
                value: 100,
                inner_struct: SomeDeserializableStruct { foo: 101 },
            },
        }
    }

    #[test]
    fn should_deserialize() {
        let expected = get_my_struct();
        let registry = get_registry();

        let input = r#"{
            "bevy_reflect::serde::de::tests::MyStruct": (
                primitive_value: 123,
                option_value: Some("Hello world!"),
                option_value_complex: Some((
                    foo: 123,
                )),
                tuple_value: (3.1415927, 1337),
                list_value: [
                    -2,
                    -1,
                    0,
                    1,
                    2,
                ],
                array_value: (-2, -1, 0, 1, 2),
                map_value: {
                    64: 32,
                },
                set_value: [
                    64,
                ],
                struct_value: (
                    foo: 999999999,
                ),
                tuple_struct_value: ("Tuple Struct"),
                unit_struct: (),
                unit_enum: Unit,
                newtype_enum: NewType(123),
                tuple_enum: Tuple(1.23, 3.21),
                struct_enum: Struct(
                    foo: "Struct variant value",
                ),
                ignored_struct: (),
                ignored_tuple_struct: (),
                ignored_struct_variant: Struct(),
                ignored_tuple_variant: Tuple(),
                custom_deserialize: (
                    value: 100,
                    renamed: (
                        foo: 101,
                    ),
                ),
            ),
        }"#;

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output = <MyStruct as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_deserialize_value() {
        let input = r#"{
            "f32": 1.23,
        }"#;

        let registry = get_registry();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();
        let output = dynamic_output
            .try_take::<f32>()
            .expect("underlying type should be f32");
        assert_eq!(1.23, output);
    }

    #[test]
    fn should_deserialized_typed() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo {
            bar: i32,
        }

        let expected = Foo { bar: 123 };

        let input = r#"(
            bar: 123
        )"#;

        let mut registry = get_registry();
        registry.register::<Foo>();
        let registration = registry.get(TypeId::of::<Foo>()).unwrap();
        let reflect_deserializer = TypedReflectDeserializer::new(registration, &registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output =
            <Foo as FromReflect>::from_reflect(dynamic_output.as_partial_reflect()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_deserialize_option() {
        #[derive(Reflect, Debug, PartialEq)]
        struct OptionTest {
            none: Option<()>,
            simple: Option<String>,
            complex: Option<SomeStruct>,
        }

        let expected = OptionTest {
            none: None,
            simple: Some(String::from("Hello world!")),
            complex: Some(SomeStruct { foo: 123 }),
        };

        let mut registry = get_registry();
        registry.register::<OptionTest>();
        registry.register::<Option<()>>();

        // === Normal === //
        let input = r#"{
            "bevy_reflect::serde::de::tests::OptionTest": (
                none: None,
                simple: Some("Hello world!"),
                complex: Some((
                    foo: 123,
                )),
            ),
        }"#;

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output = <OptionTest as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output, "failed to deserialize Options");

        // === Implicit Some === //
        let input = r#"
        #![enable(implicit_some)]
        {
            "bevy_reflect::serde::de::tests::OptionTest": (
                none: None,
                simple: "Hello world!",
                complex: (
                    foo: 123,
                ),
            ),
        }"#;

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output = <OptionTest as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(
            expected, output,
            "failed to deserialize Options with implicit Some"
        );
    }

    #[test]
    fn enum_should_deserialize() {
        #[derive(Reflect)]
        enum MyEnum {
            Unit,
            NewType(usize),
            Tuple(f32, f32),
            Struct { value: String },
        }

        let mut registry = get_registry();
        registry.register::<MyEnum>();

        // === Unit Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::MyEnum": Unit,
}"#;
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Unit);
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === NewType Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::MyEnum": NewType(123),
}"#;
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::NewType(123));
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === Tuple Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::MyEnum": Tuple(1.23, 3.21),
}"#;
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Tuple(1.23, 3.21));
        assert!(expected
            .reflect_partial_eq(output.as_partial_reflect())
            .unwrap());

        // === Struct Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::MyEnum": Struct(
        value: "I <3 Enums",
    ),
}"#;
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Struct {
            value: String::from("I <3 Enums"),
        });
        assert!(expected
            .reflect_partial_eq(output.as_partial_reflect())
            .unwrap());
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/12462
    #[test]
    fn should_reserialize() {
        let registry = get_registry();
        let input1 = get_my_struct();

        let serializer1 = ReflectSerializer::new(&input1, &registry);
        let serialized1 = ron::ser::to_string(&serializer1).unwrap();

        let mut deserializer = ron::de::Deserializer::from_str(&serialized1).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let input2 = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let serializer2 = ReflectSerializer::new(input2.as_partial_reflect(), &registry);
        let serialized2 = ron::ser::to_string(&serializer2).unwrap();

        assert_eq!(serialized1, serialized2);
    }

    #[test]
    fn should_deserialize_non_self_describing_binary() {
        let expected = get_my_struct();
        let registry = get_registry();

        let input = vec![
            1, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 98, 101, 118, 121, 95, 114, 101, 102,
            108, 101, 99, 116, 58, 58, 115, 101, 114, 100, 101, 58, 58, 100, 101, 58, 58, 116, 101,
            115, 116, 115, 58, 58, 77, 121, 83, 116, 114, 117, 99, 116, 123, 1, 12, 0, 0, 0, 0, 0,
            0, 0, 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33, 1, 123, 0, 0, 0, 0, 0,
            0, 0, 219, 15, 73, 64, 57, 5, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 254, 255, 255,
            255, 255, 255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 254, 255, 255, 255, 255,
            255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 64, 32, 0,
            0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 64, 255, 201, 154, 59, 0, 0, 0, 0, 12, 0, 0,
            0, 0, 0, 0, 0, 84, 117, 112, 108, 101, 32, 83, 116, 114, 117, 99, 116, 0, 0, 0, 0, 1,
            0, 0, 0, 123, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 164, 112, 157, 63, 164, 112, 77, 64, 3,
            0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 83, 116, 114, 117, 99, 116, 32, 118, 97, 114, 105,
            97, 110, 116, 32, 118, 97, 108, 117, 101, 1, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0,
            0, 0, 101, 0, 0, 0, 0, 0, 0, 0,
        ];

        let deserializer = ReflectDeserializer::new(&registry);

        let config = bincode::config::standard().with_fixed_int_encoding();
        let (dynamic_output, _read_bytes) =
            bincode::serde::seed_decode_from_slice(deserializer, &input, config).unwrap();

        let output = <MyStruct as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_deserialize_self_describing_binary() {
        let expected = get_my_struct();

        let registry = get_registry();

        let input = vec![
            129, 217, 40, 98, 101, 118, 121, 95, 114, 101, 102, 108, 101, 99, 116, 58, 58, 115,
            101, 114, 100, 101, 58, 58, 100, 101, 58, 58, 116, 101, 115, 116, 115, 58, 58, 77, 121,
            83, 116, 114, 117, 99, 116, 220, 0, 20, 123, 172, 72, 101, 108, 108, 111, 32, 119, 111,
            114, 108, 100, 33, 145, 123, 146, 202, 64, 73, 15, 219, 205, 5, 57, 149, 254, 255, 0,
            1, 2, 149, 254, 255, 0, 1, 2, 129, 64, 32, 145, 64, 145, 206, 59, 154, 201, 255, 172,
            84, 117, 112, 108, 101, 32, 83, 116, 114, 117, 99, 116, 144, 164, 85, 110, 105, 116,
            129, 167, 78, 101, 119, 84, 121, 112, 101, 123, 129, 165, 84, 117, 112, 108, 101, 146,
            202, 63, 157, 112, 164, 202, 64, 77, 112, 164, 129, 166, 83, 116, 114, 117, 99, 116,
            145, 180, 83, 116, 114, 117, 99, 116, 32, 118, 97, 114, 105, 97, 110, 116, 32, 118, 97,
            108, 117, 101, 144, 144, 129, 166, 83, 116, 114, 117, 99, 116, 144, 129, 165, 84, 117,
            112, 108, 101, 144, 146, 100, 145, 101,
        ];

        let mut reader = std::io::BufReader::new(input.as_slice());

        let deserializer = ReflectDeserializer::new(&registry);
        let dynamic_output = deserializer
            .deserialize(&mut rmp_serde::Deserializer::new(&mut reader))
            .unwrap();

        let output = <MyStruct as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_return_error_if_missing_type_data() {
        let mut registry = TypeRegistry::new();
        registry.register::<RangeInclusive<f32>>();

        let input = r#"{"core::ops::RangeInclusive<f32>":(start:0.0,end:1.0)}"#;
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let error = reflect_deserializer
            .deserialize(&mut deserializer)
            .unwrap_err();
        #[cfg(feature = "debug_stack")]
        assert_eq!(error, ron::Error::Message("type `core::ops::RangeInclusive<f32>` did not register the `ReflectDeserialize` type data. For certain types, this may need to be registered manually using `register_type_data` (stack: `core::ops::RangeInclusive<f32>`)".to_string()));
        #[cfg(not(feature = "debug_stack"))]
        assert_eq!(error, ron::Error::Message("type `core::ops::RangeInclusive<f32>` did not register the `ReflectDeserialize` type data. For certain types, this may need to be registered manually using `register_type_data`".to_string()));
    }

    #[test]
    fn should_use_processor_for_custom_deserialization() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo {
            bar: i32,
            qux: i64,
        }

        struct FooProcessor;

        impl ReflectDeserializerProcessor for FooProcessor {
            fn try_deserialize<'de, D>(
                &mut self,
                registration: &TypeRegistration,
                _: &TypeRegistry,
                deserializer: D,
            ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
            where
                D: Deserializer<'de>,
            {
                if registration.type_id() == TypeId::of::<i64>() {
                    let _ = deserializer.deserialize_ignored_any(IgnoredAny);
                    Ok(Ok(Box::new(456_i64)))
                } else {
                    Ok(Err(deserializer))
                }
            }
        }

        let expected = Foo { bar: 123, qux: 456 };

        let input = r#"(
            bar: 123,
            qux: 123,
        )"#;

        let mut registry = get_registry();
        registry.register::<Foo>();
        let registration = registry.get(TypeId::of::<Foo>()).unwrap();
        let mut processor = FooProcessor;
        let reflect_deserializer =
            TypedReflectDeserializer::with_processor(registration, &registry, &mut processor);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output =
            <Foo as FromReflect>::from_reflect(dynamic_output.as_partial_reflect()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_use_processor_for_multiple_registrations() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo {
            bar: i32,
            qux: i64,
        }

        struct FooProcessor;

        impl ReflectDeserializerProcessor for FooProcessor {
            fn try_deserialize<'de, D>(
                &mut self,
                registration: &TypeRegistration,
                _: &TypeRegistry,
                deserializer: D,
            ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
            where
                D: Deserializer<'de>,
            {
                if registration.type_id() == TypeId::of::<i32>() {
                    let _ = deserializer.deserialize_ignored_any(IgnoredAny);
                    Ok(Ok(Box::new(123_i32)))
                } else if registration.type_id() == TypeId::of::<i64>() {
                    let _ = deserializer.deserialize_ignored_any(IgnoredAny);
                    Ok(Ok(Box::new(456_i64)))
                } else {
                    Ok(Err(deserializer))
                }
            }
        }

        let expected = Foo { bar: 123, qux: 456 };

        let input = r#"(
            bar: 0,
            qux: 0,
        )"#;

        let mut registry = get_registry();
        registry.register::<Foo>();
        let registration = registry.get(TypeId::of::<Foo>()).unwrap();
        let mut processor = FooProcessor;
        let reflect_deserializer =
            TypedReflectDeserializer::with_processor(registration, &registry, &mut processor);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output =
            <Foo as FromReflect>::from_reflect(dynamic_output.as_partial_reflect()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_propagate_processor_deserialize_error() {
        struct ErroringProcessor;

        impl ReflectDeserializerProcessor for ErroringProcessor {
            fn try_deserialize<'de, D>(
                &mut self,
                registration: &TypeRegistration,
                _: &TypeRegistry,
                deserializer: D,
            ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
            where
                D: Deserializer<'de>,
            {
                if registration.type_id() == TypeId::of::<i32>() {
                    Err(serde::de::Error::custom("my custom deserialize error"))
                } else {
                    Ok(Err(deserializer))
                }
            }
        }

        let registry = get_registry();

        let input = r#"{"i32":123}"#;
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let mut processor = ErroringProcessor;
        let reflect_deserializer = ReflectDeserializer::with_processor(&registry, &mut processor);
        let error = reflect_deserializer
            .deserialize(&mut deserializer)
            .unwrap_err();

        #[cfg(feature = "debug_stack")]
        assert_eq!(
            error,
            ron::Error::Message("my custom deserialize error (stack: `i32`)".to_string())
        );
        #[cfg(not(feature = "debug_stack"))]
        assert_eq!(
            error,
            ron::Error::Message("my custom deserialize error".to_string())
        );
    }

    #[test]
    fn should_access_local_scope_in_processor() {
        struct ValueCountingProcessor<'a> {
            values_found: &'a mut usize,
        }

        impl ReflectDeserializerProcessor for ValueCountingProcessor<'_> {
            fn try_deserialize<'de, D>(
                &mut self,
                _: &TypeRegistration,
                _: &TypeRegistry,
                deserializer: D,
            ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
            where
                D: Deserializer<'de>,
            {
                let _ = deserializer.deserialize_ignored_any(IgnoredAny)?;
                *self.values_found += 1;
                Ok(Ok(Box::new(123_i32)))
            }
        }

        let registry = get_registry();

        let input = r#"{"i32":0}"#;

        let mut values_found = 0_usize;
        let mut deserializer_processor = ValueCountingProcessor {
            values_found: &mut values_found,
        };

        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let reflect_deserializer =
            ReflectDeserializer::with_processor(&registry, &mut deserializer_processor);
        reflect_deserializer.deserialize(&mut deserializer).unwrap();
        assert_eq!(1, values_found);
    }

    #[test]
    fn should_fail_from_reflect_if_processor_returns_wrong_typed_value() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo {
            bar: i32,
            qux: i64,
        }

        struct WrongTypeProcessor;

        impl ReflectDeserializerProcessor for WrongTypeProcessor {
            fn try_deserialize<'de, D>(
                &mut self,
                registration: &TypeRegistration,
                _registry: &TypeRegistry,
                deserializer: D,
            ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
            where
                D: Deserializer<'de>,
            {
                if registration.type_id() == TypeId::of::<i32>() {
                    let _ = deserializer.deserialize_ignored_any(IgnoredAny);
                    Ok(Ok(Box::new(42_i64)))
                } else {
                    Ok(Err(deserializer))
                }
            }
        }

        let input = r#"(
            bar: 123,
            qux: 123,
        )"#;

        let mut registry = get_registry();
        registry.register::<Foo>();
        let registration = registry.get(TypeId::of::<Foo>()).unwrap();
        let mut processor = WrongTypeProcessor;
        let reflect_deserializer =
            TypedReflectDeserializer::with_processor(registration, &registry, &mut processor);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        assert!(<Foo as FromReflect>::from_reflect(dynamic_output.as_partial_reflect()).is_none());
    }

    #[cfg(feature = "functions")]
    mod functions {
        use super::*;
        use crate::func::DynamicFunction;

        #[test]
        fn should_not_deserialize_function() {
            #[derive(Reflect)]
            #[reflect(from_reflect = false)]
            struct MyStruct {
                func: DynamicFunction<'static>,
            }

            let mut registry = TypeRegistry::new();
            registry.register::<MyStruct>();

            let input = r#"{
                "bevy_reflect::serde::de::tests::functions::MyStruct": (
                    func: (),
                ),
            }"#;

            let reflect_deserializer = ReflectDeserializer::new(&registry);
            let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();

            let error = reflect_deserializer
                .deserialize(&mut ron_deserializer)
                .unwrap_err();

            #[cfg(feature = "debug_stack")]
            assert_eq!(
                error,
                ron::Error::Message(
                    "no registration found for type `bevy_reflect::DynamicFunction` (stack: `bevy_reflect::serde::de::tests::functions::MyStruct`)"
                        .to_string()
                )
            );

            #[cfg(not(feature = "debug_stack"))]
            assert_eq!(
                error,
                ron::Error::Message(
                    "no registration found for type `bevy_reflect::DynamicFunction`".to_string()
                )
            );
        }
    }

    #[cfg(feature = "debug_stack")]
    mod debug_stack {
        use super::*;

        #[test]
        fn should_report_context_in_errors() {
            #[derive(Reflect)]
            struct Foo {
                bar: Bar,
            }

            #[derive(Reflect)]
            struct Bar {
                some_other_field: Option<u32>,
                baz: Baz,
            }

            #[derive(Reflect)]
            struct Baz {
                value: Vec<RangeInclusive<f32>>,
            }

            let mut registry = TypeRegistry::new();
            registry.register::<Foo>();
            registry.register::<Bar>();
            registry.register::<Baz>();
            registry.register::<RangeInclusive<f32>>();

            let input = r#"{"bevy_reflect::serde::de::tests::debug_stack::Foo":(bar:(some_other_field:Some(123),baz:(value:[(start:0.0,end:1.0)])))}"#;
            let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
            let reflect_deserializer = ReflectDeserializer::new(&registry);
            let error = reflect_deserializer
                .deserialize(&mut deserializer)
                .unwrap_err();
            assert_eq!(
                error,
                ron::Error::Message(
                    "type `core::ops::RangeInclusive<f32>` did not register the `ReflectDeserialize` type data. For certain types, this may need to be registered manually using `register_type_data` (stack: `bevy_reflect::serde::de::tests::debug_stack::Foo` -> `bevy_reflect::serde::de::tests::debug_stack::Bar` -> `bevy_reflect::serde::de::tests::debug_stack::Baz` -> `alloc::vec::Vec<core::ops::RangeInclusive<f32>>` -> `core::ops::RangeInclusive<f32>`)".to_string()
                )
            );
        }
    }
}
