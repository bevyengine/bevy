use crate::{
    Array, Enum, List, Map, Reflect, ReflectRef, ReflectSerialize, Struct, Tuple, TupleStruct,
    TypeInfo, TypeRegistry, VariantInfo, VariantType,
};
use serde::ser::{
    Error, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct,
    SerializeTupleVariant,
};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize,
};

use super::SerializationData;

pub enum Serializable<'a> {
    Owned(Box<dyn erased_serde::Serialize + 'a>),
    Borrowed(&'a dyn erased_serde::Serialize),
}

impl<'a> Serializable<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn borrow(&self) -> &dyn erased_serde::Serialize {
        match self {
            Serializable::Borrowed(serialize) => serialize,
            Serializable::Owned(serialize) => serialize,
        }
    }
}

fn get_serializable<'a, E: serde::ser::Error>(
    reflect_value: &'a dyn Reflect,
    type_registry: &TypeRegistry,
) -> Result<Serializable<'a>, E> {
    let reflect_serialize = type_registry
        .get_type_data::<ReflectSerialize>(reflect_value.type_id())
        .ok_or_else(|| {
            serde::ser::Error::custom(format_args!(
                "Type '{}' did not register ReflectSerialize",
                reflect_value.type_name()
            ))
        })?;
    Ok(reflect_serialize.get_serializable(reflect_value))
}

/// A general purpose serializer for reflected types.
///
/// The serialized data will take the form of a map containing the following entries:
/// 1. `type`: The _full_ [type name]
/// 2. `value`: The serialized value of the reflected type
///
/// [type name]: std::any::type_name
pub struct ReflectSerializer<'a> {
    pub value: &'a dyn Reflect,
    pub registry: &'a TypeRegistry,
}

impl<'a> ReflectSerializer<'a> {
    pub fn new(value: &'a dyn Reflect, registry: &'a TypeRegistry) -> Self {
        ReflectSerializer { value, registry }
    }
}

impl<'a> Serialize for ReflectSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(1))?;
        state.serialize_entry(
            self.value.type_name(),
            &TypedReflectSerializer::new(self.value, self.registry),
        )?;
        state.end()
    }
}

/// A serializer for reflected types whose type is known and does not require
/// serialization to include other metadata about it.
pub struct TypedReflectSerializer<'a> {
    pub value: &'a dyn Reflect,
    pub registry: &'a TypeRegistry,
}

impl<'a> TypedReflectSerializer<'a> {
    pub fn new(value: &'a dyn Reflect, registry: &'a TypeRegistry) -> Self {
        TypedReflectSerializer { value, registry }
    }
}

impl<'a> Serialize for TypedReflectSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Handle both Value case and types that have a custom `Serialize`
        let serializable = get_serializable::<S::Error>(self.value, self.registry);
        if let Ok(serializable) = serializable {
            return serializable.borrow().serialize(serializer);
        }

        match self.value.reflect_ref() {
            ReflectRef::Struct(value) => StructSerializer {
                struct_value: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::TupleStruct(value) => TupleStructSerializer {
                tuple_struct: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Tuple(value) => TupleSerializer {
                tuple: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::List(value) => ListSerializer {
                list: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Array(value) => ArraySerializer {
                array: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Map(value) => MapSerializer {
                map: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Enum(value) => EnumSerializer {
                enum_value: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Value(_) => Err(serializable.err().unwrap()),
        }
    }
}

pub struct ReflectValueSerializer<'a> {
    pub registry: &'a TypeRegistry,
    pub value: &'a dyn Reflect,
}

impl<'a> Serialize for ReflectValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        get_serializable::<S::Error>(self.value, self.registry)?
            .borrow()
            .serialize(serializer)
    }
}

pub struct StructSerializer<'a> {
    pub struct_value: &'a dyn Struct,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for StructSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let type_info = self
            .struct_value
            .get_represented_type_info()
            .ok_or_else(|| {
                Error::custom(format_args!(
                    "cannot get type info for {}",
                    self.struct_value.type_name()
                ))
            })?;

        let struct_info = match type_info {
            TypeInfo::Struct(struct_info) => struct_info,
            info => {
                return Err(Error::custom(format_args!(
                    "expected struct type but received {info:?}"
                )));
            }
        };

        let serialization_data = self
            .registry
            .get(type_info.type_id())
            .and_then(|registration| registration.data::<SerializationData>());
        let ignored_len = serialization_data.map(|data| data.len()).unwrap_or(0);
        let mut state = serializer.serialize_struct(
            struct_info.name(),
            self.struct_value.field_len() - ignored_len,
        )?;

        for (index, value) in self.struct_value.iter_fields().enumerate() {
            if serialization_data
                .map(|data| data.is_ignored_field(index))
                .unwrap_or(false)
            {
                continue;
            }
            let key = struct_info.field_at(index).unwrap().name();
            state.serialize_field(key, &TypedReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}

pub struct TupleStructSerializer<'a> {
    pub tuple_struct: &'a dyn TupleStruct,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for TupleStructSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let type_info = self
            .tuple_struct
            .get_represented_type_info()
            .ok_or_else(|| {
                Error::custom(format_args!(
                    "cannot get type info for {}",
                    self.tuple_struct.type_name()
                ))
            })?;

        let tuple_struct_info = match type_info {
            TypeInfo::TupleStruct(tuple_struct_info) => tuple_struct_info,
            info => {
                return Err(Error::custom(format_args!(
                    "expected tuple struct type but received {info:?}"
                )));
            }
        };

        let serialization_data = self
            .registry
            .get(type_info.type_id())
            .and_then(|registration| registration.data::<SerializationData>());
        let ignored_len = serialization_data.map(|data| data.len()).unwrap_or(0);
        let mut state = serializer.serialize_tuple_struct(
            tuple_struct_info.name(),
            self.tuple_struct.field_len() - ignored_len,
        )?;

        for (index, value) in self.tuple_struct.iter_fields().enumerate() {
            if serialization_data
                .map(|data| data.is_ignored_field(index))
                .unwrap_or(false)
            {
                continue;
            }
            state.serialize_field(&TypedReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}

pub struct EnumSerializer<'a> {
    pub enum_value: &'a dyn Enum,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for EnumSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let type_info = self.enum_value.get_represented_type_info().ok_or_else(|| {
            Error::custom(format_args!(
                "cannot get type info for {}",
                self.enum_value.type_name()
            ))
        })?;

        let enum_info = match type_info {
            TypeInfo::Enum(enum_info) => enum_info,
            info => {
                return Err(Error::custom(format_args!(
                    "expected enum type but received {info:?}"
                )));
            }
        };

        let enum_name = enum_info.name();
        let variant_index = self.enum_value.variant_index() as u32;
        let variant_info = enum_info
            .variant_at(variant_index as usize)
            .ok_or_else(|| {
                Error::custom(format_args!(
                    "variant at index `{variant_index}` does not exist",
                ))
            })?;
        let variant_name = variant_info.name();
        let variant_type = self.enum_value.variant_type();
        let field_len = self.enum_value.field_len();

        match variant_type {
            VariantType::Unit => {
                if self
                    .enum_value
                    .type_name()
                    .starts_with("core::option::Option")
                {
                    serializer.serialize_none()
                } else {
                    serializer.serialize_unit_variant(enum_name, variant_index, variant_name)
                }
            }
            VariantType::Struct => {
                let struct_info = match variant_info {
                    VariantInfo::Struct(struct_info) => struct_info,
                    info => {
                        return Err(Error::custom(format_args!(
                            "expected struct variant type but received {info:?}",
                        )));
                    }
                };

                let mut state = serializer.serialize_struct_variant(
                    enum_name,
                    variant_index,
                    variant_name,
                    field_len,
                )?;
                for (index, field) in self.enum_value.iter_fields().enumerate() {
                    let field_info = struct_info.field_at(index).unwrap();
                    state.serialize_field(
                        field_info.name(),
                        &TypedReflectSerializer::new(field.value(), self.registry),
                    )?;
                }
                state.end()
            }
            VariantType::Tuple if field_len == 1 => {
                let field = self.enum_value.field_at(0).unwrap();
                if self
                    .enum_value
                    .type_name()
                    .starts_with("core::option::Option")
                {
                    serializer.serialize_some(&TypedReflectSerializer::new(field, self.registry))
                } else {
                    serializer.serialize_newtype_variant(
                        enum_name,
                        variant_index,
                        variant_name,
                        &TypedReflectSerializer::new(field, self.registry),
                    )
                }
            }
            VariantType::Tuple => {
                let mut state = serializer.serialize_tuple_variant(
                    enum_name,
                    variant_index,
                    variant_name,
                    field_len,
                )?;
                for field in self.enum_value.iter_fields() {
                    state.serialize_field(&TypedReflectSerializer::new(
                        field.value(),
                        self.registry,
                    ))?;
                }
                state.end()
            }
        }
    }
}

pub struct TupleSerializer<'a> {
    pub tuple: &'a dyn Tuple,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for TupleSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_tuple(self.tuple.field_len())?;

        for value in self.tuple.iter_fields() {
            state.serialize_element(&TypedReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}

pub struct MapSerializer<'a> {
    pub map: &'a dyn Map,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for MapSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.map.len()))?;
        for (key, value) in self.map.iter() {
            state.serialize_entry(
                &TypedReflectSerializer::new(key, self.registry),
                &TypedReflectSerializer::new(value, self.registry),
            )?;
        }
        state.end()
    }
}

pub struct ListSerializer<'a> {
    pub list: &'a dyn List,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for ListSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.list.len()))?;
        for value in self.list.iter() {
            state.serialize_element(&TypedReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}

pub struct ArraySerializer<'a> {
    pub array: &'a dyn Array,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for ArraySerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_tuple(self.array.len())?;
        for value in self.array.iter() {
            state.serialize_element(&TypedReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_reflect;
    use crate::serde::ReflectSerializer;
    use crate::{Reflect, ReflectSerialize, TypeRegistry};
    use bevy_utils::HashMap;
    use ron::extensions::Extensions;
    use ron::ser::PrettyConfig;
    use serde::Serialize;
    use std::f32::consts::PI;

    #[derive(Reflect, Debug, PartialEq)]
    struct MyStruct {
        primitive_value: i8,
        option_value: Option<String>,
        option_value_complex: Option<SomeStruct>,
        tuple_value: (f32, usize),
        list_value: Vec<i32>,
        array_value: [i32; 5],
        map_value: HashMap<u8, usize>,
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

    #[test]
    fn should_serialize() {
        let mut map = HashMap::new();
        map.insert(64, 32);

        let input = MyStruct {
            primitive_value: 123,
            option_value: Some(String::from("Hello world!")),
            option_value_complex: Some(SomeStruct { foo: 123 }),
            tuple_value: (PI, 1337),
            list_value: vec![-2, -1, 0, 1, 2],
            array_value: [-2, -1, 0, 1, 2],
            map_value: map,
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
        };

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
    "bevy_reflect::serde::ser::tests::should_serialize_option::OptionTest": (
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
    "bevy_reflect::serde::ser::tests::should_serialize_option::OptionTest": (
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
    "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum": Unit,
}"#;
        assert_eq!(expected, output);

        // === NewType Variant === //
        let value = MyEnum::NewType(123);
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config.clone()).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum": NewType(123),
}"#;
        assert_eq!(expected, output);

        // === Tuple Variant === //
        let value = MyEnum::Tuple(1.23, 3.21);
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config.clone()).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum": Tuple(1.23, 3.21),
}"#;
        assert_eq!(expected, output);

        // === Struct Variant === //
        let value = MyEnum::Struct {
            value: String::from("I <3 Enums"),
        };
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config).unwrap();
        let expected = r#"{
    "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum": Struct(
        value: "I <3 Enums",
    ),
}"#;
        assert_eq!(expected, output);
    }

    #[test]
    fn should_serialize_non_self_describing_binary() {
        let mut map = HashMap::new();
        map.insert(64, 32);

        let input = MyStruct {
            primitive_value: 123,
            option_value: Some(String::from("Hello world!")),
            option_value_complex: Some(SomeStruct { foo: 123 }),
            tuple_value: (PI, 1337),
            list_value: vec![-2, -1, 0, 1, 2],
            array_value: [-2, -1, 0, 1, 2],
            map_value: map,
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
        };

        let registry = get_registry();

        let serializer = ReflectSerializer::new(&input, &registry);
        let bytes = bincode::serialize(&serializer).unwrap();

        let expected: Vec<u8> = vec![
            1, 0, 0, 0, 0, 0, 0, 0, 41, 0, 0, 0, 0, 0, 0, 0, 98, 101, 118, 121, 95, 114, 101, 102,
            108, 101, 99, 116, 58, 58, 115, 101, 114, 100, 101, 58, 58, 115, 101, 114, 58, 58, 116,
            101, 115, 116, 115, 58, 58, 77, 121, 83, 116, 114, 117, 99, 116, 123, 1, 12, 0, 0, 0,
            0, 0, 0, 0, 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33, 1, 123, 0, 0, 0,
            0, 0, 0, 0, 219, 15, 73, 64, 57, 5, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 254, 255,
            255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 254, 255, 255, 255,
            255, 255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 64, 32,
            0, 0, 0, 0, 0, 0, 0, 255, 201, 154, 59, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 84, 117,
            112, 108, 101, 32, 83, 116, 114, 117, 99, 116, 0, 0, 0, 0, 1, 0, 0, 0, 123, 0, 0, 0, 0,
            0, 0, 0, 2, 0, 0, 0, 164, 112, 157, 63, 164, 112, 77, 64, 3, 0, 0, 0, 20, 0, 0, 0, 0,
            0, 0, 0, 83, 116, 114, 117, 99, 116, 32, 118, 97, 114, 105, 97, 110, 116, 32, 118, 97,
            108, 117, 101, 1, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 101, 0, 0, 0, 0, 0, 0,
            0,
        ];

        assert_eq!(expected, bytes);
    }

    #[test]
    fn should_serialize_self_describing_binary() {
        let mut map = HashMap::new();
        map.insert(64, 32);

        let input = MyStruct {
            primitive_value: 123,
            option_value: Some(String::from("Hello world!")),
            option_value_complex: Some(SomeStruct { foo: 123 }),
            tuple_value: (PI, 1337),
            list_value: vec![-2, -1, 0, 1, 2],
            array_value: [-2, -1, 0, 1, 2],
            map_value: map,
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
        };

        let registry = get_registry();

        let serializer = ReflectSerializer::new(&input, &registry);
        let bytes: Vec<u8> = rmp_serde::to_vec(&serializer).unwrap();

        let expected: Vec<u8> = vec![
            129, 217, 41, 98, 101, 118, 121, 95, 114, 101, 102, 108, 101, 99, 116, 58, 58, 115,
            101, 114, 100, 101, 58, 58, 115, 101, 114, 58, 58, 116, 101, 115, 116, 115, 58, 58, 77,
            121, 83, 116, 114, 117, 99, 116, 220, 0, 19, 123, 172, 72, 101, 108, 108, 111, 32, 119,
            111, 114, 108, 100, 33, 145, 123, 146, 202, 64, 73, 15, 219, 205, 5, 57, 149, 254, 255,
            0, 1, 2, 149, 254, 255, 0, 1, 2, 129, 64, 32, 145, 206, 59, 154, 201, 255, 145, 172,
            84, 117, 112, 108, 101, 32, 83, 116, 114, 117, 99, 116, 144, 164, 85, 110, 105, 116,
            129, 167, 78, 101, 119, 84, 121, 112, 101, 123, 129, 165, 84, 117, 112, 108, 101, 146,
            202, 63, 157, 112, 164, 202, 64, 77, 112, 164, 129, 166, 83, 116, 114, 117, 99, 116,
            145, 180, 83, 116, 114, 117, 99, 116, 32, 118, 97, 114, 105, 97, 110, 116, 32, 118, 97,
            108, 117, 101, 144, 144, 129, 166, 83, 116, 114, 117, 99, 116, 144, 129, 165, 84, 117,
            112, 108, 101, 144, 146, 100, 145, 101,
        ];

        assert_eq!(expected, bytes);
    }
}
