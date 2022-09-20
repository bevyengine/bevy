use crate::{
    serde::type_fields, DynamicArray, DynamicEnum, DynamicList, DynamicMap, DynamicStruct,
    DynamicTuple, DynamicTupleStruct, Map, Reflect, ReflectDeserialize, TypeRegistry,
};
use erased_serde::Deserializer;
use serde::de::{self, DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};

pub trait DeserializeValue {
    fn deserialize(
        deserializer: &mut dyn Deserializer,
        type_registry: &TypeRegistry,
    ) -> Result<Box<dyn Reflect>, erased_serde::Error>;
}

pub struct ReflectDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a> ReflectDeserializer<'a> {
    pub fn new(registry: &'a TypeRegistry) -> Self {
        ReflectDeserializer { registry }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for ReflectDeserializer<'a> {
    type Value = Box<dyn Reflect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ReflectVisitor {
            registry: self.registry,
        })
    }
}

struct ReflectVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ReflectVisitor<'a> {
    type Value = Box<dyn Reflect>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflect value")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v.to_string()))
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut type_name: Option<String> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                type_fields::TYPE => {
                    type_name = Some(map.next_value()?);
                }
                type_fields::MAP => {
                    let _type_name = type_name
                        .take()
                        .ok_or_else(|| de::Error::missing_field(type_fields::TYPE))?;
                    let map = map.next_value_seed(MapDeserializer {
                        registry: self.registry,
                    })?;
                    return Ok(Box::new(map));
                }
                type_fields::STRUCT => {
                    let type_name = type_name
                        .take()
                        .ok_or_else(|| de::Error::missing_field(type_fields::TYPE))?;
                    let mut dynamic_struct = map.next_value_seed(StructDeserializer {
                        registry: self.registry,
                    })?;
                    dynamic_struct.set_name(type_name);
                    return Ok(Box::new(dynamic_struct));
                }
                type_fields::TUPLE_STRUCT => {
                    let type_name = type_name
                        .take()
                        .ok_or_else(|| de::Error::missing_field(type_fields::TYPE))?;
                    let mut tuple_struct = map.next_value_seed(TupleStructDeserializer {
                        registry: self.registry,
                    })?;
                    tuple_struct.set_name(type_name);
                    return Ok(Box::new(tuple_struct));
                }
                type_fields::TUPLE => {
                    let _type_name = type_name
                        .take()
                        .ok_or_else(|| de::Error::missing_field(type_fields::TYPE))?;
                    let tuple = map.next_value_seed(TupleDeserializer {
                        registry: self.registry,
                    })?;
                    return Ok(Box::new(tuple));
                }
                type_fields::LIST => {
                    let _type_name = type_name
                        .take()
                        .ok_or_else(|| de::Error::missing_field(type_fields::TYPE))?;
                    let list = map.next_value_seed(ListDeserializer {
                        registry: self.registry,
                    })?;
                    return Ok(Box::new(list));
                }
                type_fields::ARRAY => {
                    let _type_name = type_name
                        .take()
                        .ok_or_else(|| de::Error::missing_field(type_fields::TYPE))?;
                    let array = map.next_value_seed(ArrayDeserializer {
                        registry: self.registry,
                    })?;
                    return Ok(Box::new(array));
                }
                type_fields::ENUM => {
                    let type_name = type_name
                        .take()
                        .ok_or_else(|| de::Error::missing_field(type_fields::TYPE))?;
                    let mut dynamic_enum = map.next_value_seed(EnumDeserializer {
                        registry: self.registry,
                    })?;
                    dynamic_enum.set_name(type_name);
                    return Ok(Box::new(dynamic_enum));
                }
                type_fields::VALUE => {
                    let type_name = type_name
                        .take()
                        .ok_or_else(|| de::Error::missing_field(type_fields::TYPE))?;
                    let registration =
                        self.registry.get_with_name(&type_name).ok_or_else(|| {
                            de::Error::custom(format_args!(
                                "No registration found for {}",
                                type_name
                            ))
                        })?;
                    let deserialize_reflect =
                        registration.data::<ReflectDeserialize>().ok_or_else(|| {
                            de::Error::custom(format_args!(
                                "The TypeRegistration for {} doesn't have DeserializeReflect",
                                type_name
                            ))
                        })?;
                    let value = map.next_value_seed(DeserializeReflectDeserializer {
                        reflect_deserialize: deserialize_reflect,
                    })?;
                    return Ok(value);
                }
                _ => return Err(de::Error::unknown_field(key.as_str(), &[])),
            }
        }

        Err(de::Error::custom("Maps in this location must have the \'type\' field and one of the following fields: \'map\', \'seq\', \'value\'"))
    }
}

struct DeserializeReflectDeserializer<'a> {
    reflect_deserialize: &'a ReflectDeserialize,
}

impl<'a, 'de> DeserializeSeed<'de> for DeserializeReflectDeserializer<'a> {
    type Value = Box<dyn Reflect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self.reflect_deserialize.deserialize(deserializer)
    }
}

struct ListDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for ListDeserializer<'a> {
    type Value = DynamicList;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ListVisitor {
            registry: self.registry,
        })
    }
}

struct ListVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ListVisitor<'a> {
    type Value = DynamicList;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("list value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut list = DynamicList::default();
        while let Some(value) = seq.next_element_seed(ReflectDeserializer {
            registry: self.registry,
        })? {
            list.push_box(value);
        }
        Ok(list)
    }
}

struct ArrayDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for ArrayDeserializer<'a> {
    type Value = DynamicArray;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ArrayVisitor {
            registry: self.registry,
        })
    }
}

struct ArrayVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ArrayVisitor<'a> {
    type Value = DynamicArray;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("array value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        while let Some(value) = seq.next_element_seed(ReflectDeserializer {
            registry: self.registry,
        })? {
            vec.push(value);
        }

        Ok(DynamicArray::new(Box::from(vec)))
    }
}

struct MapDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for MapDeserializer<'a> {
    type Value = DynamicMap;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(MapVisitor {
            registry: self.registry,
        })
    }
}

struct MapVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for MapVisitor<'a> {
    type Value = DynamicMap;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("map value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut dynamic_map = DynamicMap::default();
        while let Some(key) = map.next_key_seed(ReflectDeserializer {
            registry: self.registry,
        })? {
            let value = map.next_value_seed(ReflectDeserializer {
                registry: self.registry,
            })?;
            dynamic_map.insert_boxed(key, value);
        }

        Ok(dynamic_map)
    }
}

struct StructDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for StructDeserializer<'a> {
    type Value = DynamicStruct;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(StructVisitor {
            registry: self.registry,
        })
    }
}

struct StructVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for StructVisitor<'a> {
    type Value = DynamicStruct;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut dynamic_struct = DynamicStruct::default();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(ReflectDeserializer {
                registry: self.registry,
            })?;
            dynamic_struct.insert_boxed(&key, value);
        }

        Ok(dynamic_struct)
    }
}

struct TupleStructDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for TupleStructDeserializer<'a> {
    type Value = DynamicTupleStruct;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(TupleStructVisitor {
            registry: self.registry,
        })
    }
}

struct TupleStructVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for TupleStructVisitor<'a> {
    type Value = DynamicTupleStruct;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("tuple struct value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut tuple_struct = DynamicTupleStruct::default();
        while let Some(value) = seq.next_element_seed(ReflectDeserializer {
            registry: self.registry,
        })? {
            tuple_struct.insert_boxed(value);
        }
        Ok(tuple_struct)
    }
}

struct TupleDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for TupleDeserializer<'a> {
    type Value = DynamicTuple;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(TupleVisitor {
            registry: self.registry,
        })
    }
}

struct TupleVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for TupleVisitor<'a> {
    type Value = DynamicTuple;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("tuple value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut tuple = DynamicTuple::default();
        while let Some(value) = seq.next_element_seed(ReflectDeserializer {
            registry: self.registry,
        })? {
            tuple.insert_boxed(value);
        }
        Ok(tuple)
    }
}

struct EnumDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for EnumDeserializer<'a> {
    type Value = DynamicEnum;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(EnumVisitor {
            registry: self.registry,
        })
    }
}

struct EnumVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for EnumVisitor<'a> {
    type Value = DynamicEnum;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let key = map.next_key::<String>()?;
        match key.as_deref() {
            Some(type_fields::VARIANT) => {}
            Some(key) => return Err(V::Error::unknown_field(key, &[type_fields::VARIANT])),
            _ => {
                return Err(V::Error::missing_field(type_fields::VARIANT));
            }
        }

        let variant_name = map.next_value::<String>()?;

        let mut dynamic_enum = DynamicEnum::default();

        let key = map.next_key::<String>()?;
        match key.as_deref() {
            Some(type_fields::STRUCT) => {
                let dynamic_struct = map.next_value_seed(StructDeserializer {
                    registry: self.registry,
                })?;
                dynamic_enum.set_variant(variant_name, dynamic_struct);
            }
            Some(type_fields::TUPLE) => {
                let dynamic_tuple = map.next_value_seed(TupleDeserializer {
                    registry: self.registry,
                })?;
                dynamic_enum.set_variant(variant_name, dynamic_tuple);
            }
            Some(invalid_key) => {
                return Err(V::Error::unknown_field(
                    invalid_key,
                    &[type_fields::STRUCT, type_fields::TUPLE],
                ));
            }
            None => dynamic_enum.set_variant(variant_name, ()),
        }

        Ok(dynamic_enum)
    }
}

#[cfg(test)]
mod tests {
    use super::ReflectDeserializer;
    use crate as bevy_reflect;
    use crate::prelude::*;
    use crate::{DynamicEnum, TypeRegistry};
    use ::serde::de::DeserializeSeed;

    fn get_registry() -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        registry.register::<usize>();
        registry.register::<f32>();
        registry.register::<String>();
        registry.register::<(f32, f32)>();
        registry
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
    "type": "bevy_reflect::serde::de::tests::enum_should_deserialize::MyEnum",
    "enum": {
        "variant": "Unit",
    },
}"#;
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Unit);
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === NewType Variant === //
        let input = r#"{
    "type": "bevy_reflect::serde::de::tests::enum_should_deserialize::MyEnum",
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
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::NewType(123));
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === Tuple Variant === //
        let input = r#"{
    "type": "bevy_reflect::serde::de::tests::enum_should_deserialize::MyEnum",
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
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Tuple(1.23, 3.21));
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === Struct Variant === //
        let input = r#"{
    "type": "bevy_reflect::serde::de::tests::enum_should_deserialize::MyEnum",
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
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Struct {
            value: String::from("I <3 Enums"),
        });
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());
    }
}
