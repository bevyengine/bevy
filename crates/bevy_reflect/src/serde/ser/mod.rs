pub use processor::*;
pub use serializable::*;
pub use serialize_with_registry::*;
pub use serializer::*;

mod arrays;
mod custom_serialization;
mod enums;
mod error_utils;
mod lists;
mod maps;
mod processor;
mod serializable;
mod serialize_with_registry;
mod serializer;
mod sets;
mod structs;
mod tuple_structs;
mod tuples;

#[cfg(test)]
mod tests {
    use crate::{
        serde::{ReflectSerializer, ReflectSerializerProcessor},
        PartialReflect, Reflect, ReflectSerialize, Struct, TypeRegistry,
    };
    #[cfg(feature = "functions")]
    use alloc::boxed::Box;
    use alloc::{
        string::{String, ToString},
        vec,
        vec::Vec,
    };
    use bevy_platform::collections::{HashMap, HashSet};
    use core::{any::TypeId, f32::consts::PI, ops::RangeInclusive};
    use ron::{extensions::Extensions, ser::PrettyConfig};
    use serde::{Serialize, Serializer};

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
        custom_serialize: CustomSerialize,
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

    #[derive(Reflect, Debug, PartialEq, Serialize)]
    struct SomeSerializableStruct {
        foo: i64,
    }

    /// Implements a custom serialize using `#[reflect(Serialize)]`.
    ///
    /// For testing purposes, this just uses the generated one from deriving Serialize.
    #[derive(Reflect, Debug, PartialEq, Serialize)]
    #[reflect(Serialize)]
    struct CustomSerialize {
        value: usize,
        #[serde(rename = "renamed")]
        inner_struct: SomeSerializableStruct,
    }

    fn get_registry() -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        registry.register::<MyStruct>();
        registry.register::<SomeStruct>();
        registry.register::<SomeTupleStruct>();
        registry.register::<SomeUnitStruct>();
        registry.register::<SomeIgnoredStruct>();
        registry.register::<SomeIgnoredTupleStruct>();
        registry.register::<SomeIgnoredEnum>();
        registry.register::<CustomSerialize>();
        registry.register::<SomeEnum>();
        registry.register::<SomeSerializableStruct>();
        registry.register_type_data::<SomeSerializableStruct, ReflectSerialize>();
        registry.register::<String>();
        registry.register::<Option<String>>();
        registry.register_type_data::<Option<String>, ReflectSerialize>();
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
            ignored_struct: SomeIgnoredStruct { ignored: 123 },
            ignored_tuple_struct: SomeIgnoredTupleStruct(123),
            ignored_struct_variant: SomeIgnoredEnum::Struct {
                foo: String::from("Struct Variant"),
            },
            ignored_tuple_variant: SomeIgnoredEnum::Tuple(1.23, 3.45),
            custom_serialize: CustomSerialize {
                value: 100,
                inner_struct: SomeSerializableStruct { foo: 101 },
            },
        }
    }

    #[test]
    fn should_serialize() {
        let input = get_my_struct();
        let registry = get_registry();

        let serializer = ReflectSerializer::new(&input, &registry);

        let config = PrettyConfig::default()
            .new_line(String::from("\n"))
            .indentor(String::from("    "));

        let output = ron::ser::to_string_pretty(&serializer, config).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::MyStruct": (
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
        custom_serialize: (
            value: 100,
            renamed: (
                foo: 101,
            ),
        ),
    ),
}"#;
        assert_eq!(expected, output);
    }

    #[test]
    fn should_serialize_option() {
        #[derive(Reflect, Debug, PartialEq)]
        struct OptionTest {
            none: Option<()>,
            simple: Option<String>,
            complex: Option<SomeStruct>,
        }

        let value = OptionTest {
            none: None,
            simple: Some(String::from("Hello world!")),
            complex: Some(SomeStruct { foo: 123 }),
        };

        let registry = get_registry();
        let serializer = ReflectSerializer::new(&value, &registry);

        // === Normal === //
        let config = PrettyConfig::default()
            .new_line(String::from("\n"))
            .indentor(String::from("    "));

        let output = ron::ser::to_string_pretty(&serializer, config).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::OptionTest": (
        none: None,
        simple: Some("Hello world!"),
        complex: Some((
            foo: 123,
        )),
    ),
}"#;

        assert_eq!(expected, output);

        // === Implicit Some === //
        let config = PrettyConfig::default()
            .new_line(String::from("\n"))
            .extensions(Extensions::IMPLICIT_SOME)
            .indentor(String::from("    "));

        let output = ron::ser::to_string_pretty(&serializer, config).unwrap();
        let expected = r#"#![enable(implicit_some)]
{
    "bevy_reflect::serde::ser::tests::OptionTest": (
        none: None,
        simple: "Hello world!",
        complex: (
            foo: 123,
        ),
    ),
}"#;

        assert_eq!(expected, output);
    }

    #[test]
    fn enum_should_serialize() {
        #[derive(Reflect)]
        enum MyEnum {
            Unit,
            NewType(usize),
            Tuple(f32, f32),
            Struct { value: String },
        }

        let mut registry = get_registry();
        registry.register::<MyEnum>();

        let config = PrettyConfig::default().new_line(String::from("\n"));

        // === Unit Variant === //
        let value = MyEnum::Unit;
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config.clone()).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::MyEnum": Unit,
}"#;
        assert_eq!(expected, output);

        // === NewType Variant === //
        let value = MyEnum::NewType(123);
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config.clone()).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::MyEnum": NewType(123),
}"#;
        assert_eq!(expected, output);

        // === Tuple Variant === //
        let value = MyEnum::Tuple(1.23, 3.21);
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config.clone()).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::MyEnum": Tuple(1.23, 3.21),
}"#;
        assert_eq!(expected, output);

        // === Struct Variant === //
        let value = MyEnum::Struct {
            value: String::from("I <3 Enums"),
        };
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::MyEnum": Struct(
        value: "I <3 Enums",
    ),
}"#;
        assert_eq!(expected, output);
    }

    #[test]
    fn should_serialize_non_self_describing_binary() {
        let input = get_my_struct();
        let registry = get_registry();

        let serializer = ReflectSerializer::new(&input, &registry);
        let config = bincode::config::standard().with_fixed_int_encoding();
        let bytes = bincode::serde::encode_to_vec(&serializer, config).unwrap();

        let expected: Vec<u8> = vec![
            1, 0, 0, 0, 0, 0, 0, 0, 41, 0, 0, 0, 0, 0, 0, 0, 98, 101, 118, 121, 95, 114, 101, 102,
            108, 101, 99, 116, 58, 58, 115, 101, 114, 100, 101, 58, 58, 115, 101, 114, 58, 58, 116,
            101, 115, 116, 115, 58, 58, 77, 121, 83, 116, 114, 117, 99, 116, 123, 1, 12, 0, 0, 0,
            0, 0, 0, 0, 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33, 1, 123, 0, 0, 0,
            0, 0, 0, 0, 219, 15, 73, 64, 57, 5, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 254, 255,
            255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 254, 255, 255, 255,
            255, 255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 64, 32,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 64, 255, 201, 154, 59, 0, 0, 0, 0, 12, 0,
            0, 0, 0, 0, 0, 0, 84, 117, 112, 108, 101, 32, 83, 116, 114, 117, 99, 116, 0, 0, 0, 0,
            1, 0, 0, 0, 123, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 164, 112, 157, 63, 164, 112, 77, 64,
            3, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 83, 116, 114, 117, 99, 116, 32, 118, 97, 114, 105,
            97, 110, 116, 32, 118, 97, 108, 117, 101, 1, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0,
            0, 0, 101, 0, 0, 0, 0, 0, 0, 0,
        ];

        assert_eq!(expected, bytes);
    }

    #[test]
    fn should_serialize_self_describing_binary() {
        let input = get_my_struct();
        let registry = get_registry();

        let serializer = ReflectSerializer::new(&input, &registry);
        let bytes: Vec<u8> = rmp_serde::to_vec(&serializer).unwrap();

        let expected: Vec<u8> = vec![
            129, 217, 41, 98, 101, 118, 121, 95, 114, 101, 102, 108, 101, 99, 116, 58, 58, 115,
            101, 114, 100, 101, 58, 58, 115, 101, 114, 58, 58, 116, 101, 115, 116, 115, 58, 58, 77,
            121, 83, 116, 114, 117, 99, 116, 220, 0, 20, 123, 172, 72, 101, 108, 108, 111, 32, 119,
            111, 114, 108, 100, 33, 145, 123, 146, 202, 64, 73, 15, 219, 205, 5, 57, 149, 254, 255,
            0, 1, 2, 149, 254, 255, 0, 1, 2, 129, 64, 32, 145, 64, 145, 206, 59, 154, 201, 255,
            172, 84, 117, 112, 108, 101, 32, 83, 116, 114, 117, 99, 116, 144, 164, 85, 110, 105,
            116, 129, 167, 78, 101, 119, 84, 121, 112, 101, 123, 129, 165, 84, 117, 112, 108, 101,
            146, 202, 63, 157, 112, 164, 202, 64, 77, 112, 164, 129, 166, 83, 116, 114, 117, 99,
            116, 145, 180, 83, 116, 114, 117, 99, 116, 32, 118, 97, 114, 105, 97, 110, 116, 32,
            118, 97, 108, 117, 101, 144, 144, 129, 166, 83, 116, 114, 117, 99, 116, 144, 129, 165,
            84, 117, 112, 108, 101, 144, 146, 100, 145, 101,
        ];

        assert_eq!(expected, bytes);
    }

    #[test]
    fn should_serialize_dynamic_option() {
        #[derive(Default, Reflect)]
        struct OtherStruct {
            some: Option<SomeStruct>,
            none: Option<SomeStruct>,
        }

        let value = OtherStruct {
            some: Some(SomeStruct { foo: 999999999 }),
            none: None,
        };
        let dynamic = value.to_dynamic_struct();
        let reflect = dynamic.as_partial_reflect();

        let registry = get_registry();

        let serializer = ReflectSerializer::new(reflect, &registry);

        let mut buf = Vec::new();

        let format = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, format);

        serializer.serialize(&mut ser).unwrap();

        let output = core::str::from_utf8(&buf).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::OtherStruct": {
        "some": {
            "foo": 999999999
        },
        "none": null
    }
}"#;

        assert_eq!(expected, output);
    }

    #[test]
    fn should_return_error_if_missing_registration() {
        let value = RangeInclusive::<f32>::new(0.0, 1.0);
        let registry = TypeRegistry::new();

        let serializer = ReflectSerializer::new(&value, &registry);
        let error = ron::ser::to_string(&serializer).unwrap_err();
        #[cfg(feature = "debug_stack")]
        assert_eq!(
            error,
            ron::Error::Message(
                "type `core::ops::RangeInclusive<f32>` is not registered in the type registry (stack: `core::ops::RangeInclusive<f32>`)"
                    .to_string(),
            )
        );
        #[cfg(not(feature = "debug_stack"))]
        assert_eq!(
            error,
            ron::Error::Message(
                "type `core::ops::RangeInclusive<f32>` is not registered in the type registry"
                    .to_string(),
            )
        );
    }

    #[test]
    fn should_return_error_if_missing_type_data() {
        let value = RangeInclusive::<f32>::new(0.0, 1.0);
        let mut registry = TypeRegistry::new();
        registry.register::<RangeInclusive<f32>>();

        let serializer = ReflectSerializer::new(&value, &registry);
        let error = ron::ser::to_string(&serializer).unwrap_err();
        #[cfg(feature = "debug_stack")]
        assert_eq!(
            error,
            ron::Error::Message(
                "type `core::ops::RangeInclusive<f32>` did not register the `ReflectSerialize` or `ReflectSerializeWithRegistry` type data. For certain types, this may need to be registered manually using `register_type_data` (stack: `core::ops::RangeInclusive<f32>`)".to_string()
            )
        );
        #[cfg(not(feature = "debug_stack"))]
        assert_eq!(
            error,
            ron::Error::Message(
                "type `core::ops::RangeInclusive<f32>` did not register the `ReflectSerialize` type data. For certain types, this may need to be registered manually using `register_type_data`".to_string()
            )
        );
    }

    #[test]
    fn should_use_processor_for_custom_serialization() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo {
            bar: i32,
            qux: i64,
        }

        struct FooProcessor;

        impl ReflectSerializerProcessor for FooProcessor {
            fn try_serialize<S>(
                &self,
                value: &dyn PartialReflect,
                _: &TypeRegistry,
                serializer: S,
            ) -> Result<Result<S::Ok, S>, S::Error>
            where
                S: Serializer,
            {
                let Some(value) = value.try_as_reflect() else {
                    return Ok(Err(serializer));
                };

                let type_id = value.reflect_type_info().type_id();
                if type_id == TypeId::of::<i64>() {
                    Ok(Ok(serializer.serialize_str("custom!")?))
                } else {
                    Ok(Err(serializer))
                }
            }
        }

        let value = Foo { bar: 123, qux: 456 };

        let mut registry = TypeRegistry::new();
        registry.register::<Foo>();

        let processor = FooProcessor;
        let serializer = ReflectSerializer::with_processor(&value, &registry, &processor);

        let config = PrettyConfig::default().new_line(String::from("\n"));
        let output = ron::ser::to_string_pretty(&serializer, config).unwrap();

        let expected = r#"{
    "bevy_reflect::serde::ser::tests::Foo": (
        bar: 123,
        qux: "custom!",
    ),
}"#;

        assert_eq!(expected, output);
    }

    #[test]
    fn should_use_processor_for_multiple_registrations() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo {
            bar: i32,
            sub: SubFoo,
        }

        #[derive(Reflect, Debug, PartialEq)]
        struct SubFoo {
            val: i32,
        }

        struct FooProcessor;

        impl ReflectSerializerProcessor for FooProcessor {
            fn try_serialize<S>(
                &self,
                value: &dyn PartialReflect,
                _: &TypeRegistry,
                serializer: S,
            ) -> Result<Result<S::Ok, S>, S::Error>
            where
                S: Serializer,
            {
                let Some(value) = value.try_as_reflect() else {
                    return Ok(Err(serializer));
                };

                let type_id = value.reflect_type_info().type_id();
                if type_id == TypeId::of::<i32>() {
                    Ok(Ok(serializer.serialize_str("an i32")?))
                } else if type_id == TypeId::of::<SubFoo>() {
                    Ok(Ok(serializer.serialize_str("a SubFoo")?))
                } else {
                    Ok(Err(serializer))
                }
            }
        }

        let value = Foo {
            bar: 123,
            sub: SubFoo { val: 456 },
        };

        let mut registry = TypeRegistry::new();
        registry.register::<Foo>();
        registry.register::<SubFoo>();

        let processor = FooProcessor;
        let serializer = ReflectSerializer::with_processor(&value, &registry, &processor);

        let config = PrettyConfig::default().new_line(String::from("\n"));
        let output = ron::ser::to_string_pretty(&serializer, config).unwrap();

        let expected = r#"{
    "bevy_reflect::serde::ser::tests::Foo": (
        bar: "an i32",
        sub: "a SubFoo",
    ),
}"#;

        assert_eq!(expected, output);
    }

    #[test]
    fn should_propagate_processor_serialize_error() {
        struct ErroringProcessor;

        impl ReflectSerializerProcessor for ErroringProcessor {
            fn try_serialize<S>(
                &self,
                value: &dyn PartialReflect,
                _: &TypeRegistry,
                serializer: S,
            ) -> Result<Result<S::Ok, S>, S::Error>
            where
                S: Serializer,
            {
                let Some(value) = value.try_as_reflect() else {
                    return Ok(Err(serializer));
                };

                let type_id = value.reflect_type_info().type_id();
                if type_id == TypeId::of::<i32>() {
                    Err(serde::ser::Error::custom("my custom serialize error"))
                } else {
                    Ok(Err(serializer))
                }
            }
        }

        let value = 123_i32;

        let registry = TypeRegistry::new();

        let processor = ErroringProcessor;
        let serializer = ReflectSerializer::with_processor(&value, &registry, &processor);
        let error = ron::ser::to_string_pretty(&serializer, PrettyConfig::default()).unwrap_err();

        #[cfg(feature = "debug_stack")]
        assert_eq!(
            error,
            ron::Error::Message("my custom serialize error (stack: `i32`)".to_string())
        );
        #[cfg(not(feature = "debug_stack"))]
        assert_eq!(
            error,
            ron::Error::Message("my custom serialize error".to_string())
        );
    }

    #[cfg(feature = "functions")]
    mod functions {
        use super::*;
        use crate::func::{DynamicFunction, IntoFunction};
        use alloc::string::ToString;

        #[test]
        fn should_not_serialize_function() {
            #[derive(Reflect)]
            #[reflect(from_reflect = false)]
            struct MyStruct {
                func: DynamicFunction<'static>,
            }

            let value: Box<dyn Reflect> = Box::new(MyStruct {
                func: String::new.into_function(),
            });

            let registry = TypeRegistry::new();
            let serializer = ReflectSerializer::new(value.as_partial_reflect(), &registry);

            let error = ron::ser::to_string(&serializer).unwrap_err();

            #[cfg(feature = "debug_stack")]
            assert_eq!(
                error,
                ron::Error::Message("functions cannot be serialized (stack: `bevy_reflect::serde::ser::tests::functions::MyStruct`)".to_string())
            );

            #[cfg(not(feature = "debug_stack"))]
            assert_eq!(
                error,
                ron::Error::Message("functions cannot be serialized".to_string())
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

            let value = Foo {
                bar: Bar {
                    some_other_field: Some(123),
                    baz: Baz {
                        value: vec![0.0..=1.0],
                    },
                },
            };

            let registry = TypeRegistry::new();
            let serializer = ReflectSerializer::new(&value, &registry);

            let error = ron::ser::to_string(&serializer).unwrap_err();
            assert_eq!(
                error,
                ron::Error::Message(
                    "type `core::ops::RangeInclusive<f32>` is not registered in the type registry (stack: `bevy_reflect::serde::ser::tests::debug_stack::Foo` -> `bevy_reflect::serde::ser::tests::debug_stack::Bar` -> `bevy_reflect::serde::ser::tests::debug_stack::Baz` -> `alloc::vec::Vec<core::ops::RangeInclusive<f32>>` -> `core::ops::RangeInclusive<f32>`)".to_string()
                )
            );
        }
    }
}
