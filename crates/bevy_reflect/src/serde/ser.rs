use crate::{
    serde::type_fields, Array, Enum, List, Map, Reflect, ReflectRef, ReflectSerialize, Struct,
    Tuple, TupleStruct, TypeRegistry, VariantType,
};
use serde::ser::SerializeTuple;
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};

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
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry(type_fields::TYPE, self.value.type_name())?;
        state.serialize_entry(
            type_fields::VALUE,
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
        let mut state = serializer.serialize_map(Some(self.struct_value.field_len()))?;
        for (index, value) in self.struct_value.iter_fields().enumerate() {
            let key = self.struct_value.name_at(index).unwrap();
            state.serialize_entry(key, &TypedReflectSerializer::new(value, self.registry))?;
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
        let mut state = serializer.serialize_tuple(self.tuple_struct.field_len())?;
        for value in self.tuple_struct.iter_fields() {
            state.serialize_element(&TypedReflectSerializer::new(value, self.registry))?;
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
        let mut state = serializer.serialize_map(Some(1))?;
        state.serialize_entry(
            self.enum_value.variant_name(),
            &EnumValueSerializer {
                enum_value: self.enum_value,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct EnumValueSerializer<'a> {
    pub enum_value: &'a dyn Enum,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for EnumValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.enum_value.variant_type() {
            VariantType::Struct => {
                let mut state = serializer.serialize_map(Some(self.enum_value.field_len()))?;
                for field in self.enum_value.iter_fields() {
                    let key = field.name().expect("named field");
                    let value = TypedReflectSerializer::new(field.value(), self.registry);
                    state.serialize_entry(key, &value)?;
                }
                state.end()
            }
            VariantType::Tuple => {
                let mut state = serializer.serialize_tuple(self.enum_value.field_len())?;
                for field in self.enum_value.iter_fields() {
                    let value = TypedReflectSerializer::new(field.value(), self.registry);
                    state.serialize_element(&value)?;
                }
                state.end()
            }
            VariantType::Unit => serializer.serialize_unit(),
        }
    }
}

pub struct TupleVariantSerializer<'a> {
    pub enum_value: &'a dyn Enum,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for TupleVariantSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_tuple(self.enum_value.field_len())?;
        for field in self.enum_value.iter_fields() {
            let value = TypedReflectSerializer::new(field.value(), self.registry);
            state.serialize_element(&value)?;
        }
        state.end()
    }
}

pub struct StructVariantSerializer<'a> {
    pub enum_value: &'a dyn Enum,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for StructVariantSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.enum_value.field_len()))?;
        for field in self.enum_value.iter_fields() {
            let key = field.name().expect("named field");
            let value = TypedReflectSerializer::new(field.value(), self.registry);
            state.serialize_entry(key, &value)?;
        }
        state.end()
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
    use ron::ser::PrettyConfig;
    use serde::Serialize;
    use std::f32::consts::PI;

    #[derive(Reflect, Debug, PartialEq)]
    struct MyStruct {
        primitive_value: i8,
        option_value: Option<String>,
        tuple_value: (f32, usize),
        list_value: Vec<i32>,
        array_value: [i32; 5],
        map_value: HashMap<u8, usize>,
        struct_value: SomeStruct,
        tuple_struct_value: SomeTupleStruct,
        custom_serialize: CustomSerialize,
    }

    #[derive(Reflect, Debug, PartialEq, Serialize)]
    struct SomeStruct {
        foo: i64,
    }

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeTupleStruct(String);

    /// Implements a custom serialize using `#[reflect(Serialize)]`.
    ///
    /// For testing purposes, this just uses the generated one from deriving Serialize.
    #[derive(Reflect, Debug, PartialEq, Serialize)]
    #[reflect(Serialize)]
    struct CustomSerialize {
        value: usize,
        #[serde(rename = "renamed")]
        inner_struct: SomeStruct,
    }

    fn get_registry() -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        registry.register::<MyStruct>();
        registry.register::<SomeStruct>();
        registry.register::<SomeTupleStruct>();
        registry.register::<CustomSerialize>();
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
            tuple_value: (PI, 1337),
            list_value: vec![-2, -1, 0, 1, 2],
            array_value: [-2, -1, 0, 1, 2],
            map_value: map,
            struct_value: SomeStruct { foo: 999999999 },
            tuple_struct_value: SomeTupleStruct(String::from("Tuple Struct")),
            custom_serialize: CustomSerialize {
                value: 100,
                inner_struct: SomeStruct { foo: 101 },
            },
        };

        let registry = get_registry();
        let serializer = ReflectSerializer::new(&input, &registry);

        let config = PrettyConfig::default().new_line(String::from("\n"));

        let output = ron::ser::to_string_pretty(&serializer, config).unwrap();
        let expected = r#"{
    "type": "bevy_reflect::serde::ser::tests::MyStruct",
    "value": {
        "primitive_value": 123,
        "option_value": Some("Hello world!"),
        "tuple_value": (3.1415927, 1337),
        "list_value": [
            -2,
            -1,
            0,
            1,
            2,
        ],
        "array_value": (-2, -1, 0, 1, 2),
        "map_value": {
            64: 32,
        },
        "struct_value": {
            "foo": 999999999,
        },
        "tuple_struct_value": ("Tuple Struct"),
        "custom_serialize": (
            value: 100,
            renamed: (
                foo: 101,
            ),
        ),
    },
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
    "type": "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum",
    "value": {
        "Unit": (),
    },
}"#;
        assert_eq!(expected, output);

        // === NewType Variant === //
        let value = MyEnum::NewType(123);
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config.clone()).unwrap();
        let expected = r#"{
    "type": "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum",
    "value": {
        "NewType": (123),
    },
}"#;
        assert_eq!(expected, output);

        // === Tuple Variant === //
        let value = MyEnum::Tuple(1.23, 3.21);
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config.clone()).unwrap();
        let expected = r#"{
    "type": "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum",
    "value": {
        "Tuple": (1.23, 3.21),
    },
}"#;
        assert_eq!(expected, output);

        // === Struct Variant === //
        let value = MyEnum::Struct {
            value: String::from("I <3 Enums"),
        };
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config).unwrap();
        let expected = r#"{
    "type": "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum",
    "value": {
        "Struct": {
            "value": "I <3 Enums",
        },
    },
}"#;
        assert_eq!(expected, output);
    }
}
