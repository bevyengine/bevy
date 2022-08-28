use crate::{
    serde::type_fields, Array, Enum, List, Map, Reflect, ReflectRef, ReflectSerialize, Struct,
    Tuple, TupleStruct, TypeRegistry, VariantType,
};
use serde::ser::Error;
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
            ReflectRef::Value(value) => ReflectValueSerializer {
                registry: self.registry,
                value,
            }
            .serialize(serializer),
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
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry(type_fields::TYPE, self.value.type_name())?;
        state.serialize_entry(
            type_fields::VALUE,
            get_serializable::<S::Error>(self.value, self.registry)?.borrow(),
        )?;
        state.end()
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
        let mut state = serializer.serialize_map(Some(2))?;

        state.serialize_entry(type_fields::TYPE, self.struct_value.type_name())?;
        state.serialize_entry(
            type_fields::STRUCT,
            &StructValueSerializer {
                struct_value: self.struct_value,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct StructValueSerializer<'a> {
    pub struct_value: &'a dyn Struct,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for StructValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.struct_value.field_len()))?;
        for (index, value) in self.struct_value.iter_fields().enumerate() {
            let key = self.struct_value.name_at(index).unwrap();
            state.serialize_entry(key, &ReflectSerializer::new(value, self.registry))?;
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
        let mut state = serializer.serialize_map(Some(2))?;

        state.serialize_entry(type_fields::TYPE, self.tuple_struct.type_name())?;
        state.serialize_entry(
            type_fields::TUPLE_STRUCT,
            &TupleStructValueSerializer {
                tuple_struct: self.tuple_struct,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct TupleStructValueSerializer<'a> {
    pub tuple_struct: &'a dyn TupleStruct,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for TupleStructValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.tuple_struct.field_len()))?;
        for value in self.tuple_struct.iter_fields() {
            state.serialize_element(&ReflectSerializer::new(value, self.registry))?;
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
        let mut state = serializer.serialize_map(Some(2))?;

        state.serialize_entry(type_fields::TYPE, self.enum_value.type_name())?;
        state.serialize_entry(
            type_fields::ENUM,
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
        let variant_type = self.enum_value.variant_type();
        let variant_name = self.enum_value.variant_name();

        let mut state = if matches!(variant_type, VariantType::Unit) {
            serializer.serialize_map(Some(1))?
        } else {
            serializer.serialize_map(Some(2))?
        };

        state.serialize_entry(type_fields::VARIANT, variant_name)?;

        match self.enum_value.variant_type() {
            VariantType::Struct => {
                state.serialize_key(type_fields::STRUCT)?;
                state.serialize_value(&StructVariantSerializer {
                    enum_value: self.enum_value,
                    registry: self.registry,
                })?;
            }
            VariantType::Tuple => {
                state.serialize_key(type_fields::TUPLE)?;
                state.serialize_value(&TupleVariantSerializer {
                    enum_value: self.enum_value,
                    registry: self.registry,
                })?;
            }
            _ => {}
        }

        state.end()
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
        let field_len = self.enum_value.field_len();
        let mut state = serializer.serialize_seq(Some(field_len))?;
        for field in self.enum_value.iter_fields() {
            state.serialize_element(&ReflectSerializer::new(field.value(), self.registry))?;
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
        let field_len = self.enum_value.field_len();
        let mut state = serializer.serialize_map(Some(field_len))?;
        for (index, field) in self.enum_value.iter_fields().enumerate() {
            let name = field.name().ok_or_else(|| {
                S::Error::custom(format_args!(
                    "struct variant missing name for field at index {}",
                    index
                ))
            })?;
            state.serialize_entry(name, &ReflectSerializer::new(field.value(), self.registry))?;
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
        let mut state = serializer.serialize_map(Some(2))?;

        state.serialize_entry(type_fields::TYPE, self.tuple.type_name())?;
        state.serialize_entry(
            type_fields::TUPLE,
            &TupleValueSerializer {
                tuple: self.tuple,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct TupleValueSerializer<'a> {
    pub tuple: &'a dyn Tuple,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for TupleValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.tuple.field_len()))?;
        for value in self.tuple.iter_fields() {
            state.serialize_element(&ReflectSerializer::new(value, self.registry))?;
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
        let mut state = serializer.serialize_map(Some(2))?;

        state.serialize_entry(type_fields::TYPE, self.map.type_name())?;
        state.serialize_entry(
            type_fields::MAP,
            &MapValueSerializer {
                map: self.map,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct MapValueSerializer<'a> {
    pub map: &'a dyn Map,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for MapValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.map.len()))?;
        for (key, value) in self.map.iter() {
            state.serialize_entry(
                &ReflectSerializer::new(key, self.registry),
                &ReflectSerializer::new(value, self.registry),
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
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry(type_fields::TYPE, self.list.type_name())?;
        state.serialize_entry(
            type_fields::LIST,
            &ListValueSerializer {
                list: self.list,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct ListValueSerializer<'a> {
    pub list: &'a dyn List,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for ListValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.list.len()))?;
        for value in self.list.iter() {
            state.serialize_element(&ReflectSerializer::new(value, self.registry))?;
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
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry(type_fields::TYPE, self.array.type_name())?;
        state.serialize_entry(
            type_fields::ARRAY,
            &ArrayValueSerializer {
                array: self.array,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct ArrayValueSerializer<'a> {
    pub array: &'a dyn Array,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for ArrayValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.array.len()))?;
        for value in self.array.iter() {
            state.serialize_element(&ReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::ReflectSerializer;
    use crate as bevy_reflect;
    use crate::prelude::*;
    use crate::TypeRegistry;
    use ron::ser::PrettyConfig;

    fn get_registry() -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        registry.register::<usize>();
        registry.register::<f32>();
        registry.register::<String>();
        registry.register::<(f32, f32)>();
        registry
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
    "enum": {
        "variant": "Unit",
    },
}"#;
        assert_eq!(expected, output);

        // === NewType Variant === //
        let value = MyEnum::NewType(123);
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config.clone()).unwrap();
        let expected = r#"{
    "type": "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum",
    "enum": {
        "variant": "NewType",
        "tuple": [
            {
                "type": "usize",
                "value": 123,
            },
        ],
    },
}"#;
        assert_eq!(expected, output);

        // === Tuple Variant === //
        let value = MyEnum::Tuple(1.23, 3.21);
        let serializer = ReflectSerializer::new(&value, &registry);
        let output = ron::ser::to_string_pretty(&serializer, config.clone()).unwrap();
        let expected = r#"{
    "type": "bevy_reflect::serde::ser::tests::enum_should_serialize::MyEnum",
    "enum": {
        "variant": "Tuple",
        "tuple": [
            {
                "type": "f32",
                "value": 1.23,
            },
            {
                "type": "f32",
                "value": 3.21,
            },
        ],
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
    "enum": {
        "variant": "Struct",
        "struct": {
            "value": {
                "type": "alloc::string::String",
                "value": "I <3 Enums",
            },
        },
    },
}"#;
        assert_eq!(expected, output.replace('\r', ""));
    }
}
