use crate::{
    serde::type_fields, ArrayInfo, DynamicArray, DynamicEnum, DynamicList, DynamicMap,
    DynamicStruct, DynamicTuple, DynamicTupleStruct, EnumInfo, ListInfo, Map, MapInfo, NamedField,
    Reflect, ReflectDeserialize, StructInfo, StructVariantInfo, Tuple, TupleInfo, TupleStruct,
    TupleStructInfo, TupleVariantInfo, TypeInfo, TypeRegistry, UnnamedField, VariantInfo,
};
use erased_serde::Deserializer;
use serde::de::{self, DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use std::any::TypeId;
use std::fmt::Formatter;

pub trait DeserializeValue {
    fn deserialize(
        deserializer: &mut dyn Deserializer,
        type_registry: &TypeRegistry,
    ) -> Result<Box<dyn Reflect>, erased_serde::Error>;
}

trait StructLikeInfo {
    fn get_name(&self) -> &str;
    fn get_field(&self, name: &str) -> Option<&NamedField>;
}

trait TupleLikeInfo {
    fn get_name(&self) -> &str;
    fn get_field(&self, index: usize) -> Option<&UnnamedField>;
    fn get_field_len(&self) -> usize;
}

impl StructLikeInfo for StructInfo {
    fn get_name(&self) -> &str {
        self.type_name()
    }

    fn get_field(&self, name: &str) -> Option<&NamedField> {
        self.field(name)
    }
}

impl StructLikeInfo for StructVariantInfo {
    fn get_name(&self) -> &str {
        self.name()
    }

    fn get_field(&self, name: &str) -> Option<&NamedField> {
        self.field(name)
    }
}

impl TupleLikeInfo for TupleInfo {
    fn get_name(&self) -> &str {
        self.type_name()
    }

    fn get_field(&self, index: usize) -> Option<&UnnamedField> {
        self.field_at(index)
    }

    fn get_field_len(&self) -> usize {
        self.field_len()
    }
}

impl TupleLikeInfo for TupleVariantInfo {
    fn get_name(&self) -> &str {
        self.name()
    }

    fn get_field(&self, index: usize) -> Option<&UnnamedField> {
        self.field_at(index)
    }

    fn get_field_len(&self) -> usize {
        self.field_len()
    }
}

/// A general purpose deserializer for reflected types.
///
/// For non-value types, this will return the dynamic equivalent. For example, a
/// deserialized struct will return a [`DynamicStruct`] and a `Vec` will return a
/// [`DynamicList`].
///
/// The serialized data must take the form of a map containing the following entries:
/// 1. `type`: The _full_ [type name]
/// 2. `value`: The serialized value of the reflected type
///
/// > Note: The ordering is important here. `type` _must_ come before `value`.
///
/// [`DynamicStruct`]: crate::DynamicStruct
/// [`DynamicList`]: crate::DynamicList
/// [type name]: std::any::type_name
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
        deserializer.deserialize_map(ReflectDeserializerVisitor {
            registry: self.registry,
        })
    }
}

struct ReflectDeserializerVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ReflectDeserializerVisitor<'a> {
    type Value = Box<dyn Reflect>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("map containing `type` and `value` entries for the reflected value")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let type_name = match map.next_key::<&str>()? {
            Some(type_fields::TYPE) => map.next_value::<&str>()?,
            Some(type_fields::VALUE) => {
                // `type` must come before `value`.
                return Err(de::Error::missing_field(type_fields::TYPE));
            }
            Some(field) => {
                return Err(de::Error::unknown_field(field, &[type_fields::TYPE]));
            }
            None => {
                return Err(de::Error::invalid_length(
                    0,
                    &"two entries: `type` and `value`",
                ));
            }
        };

        match map.next_key::<&str>()? {
            Some(type_fields::VALUE) => {
                let registration = self.registry.get_with_name(type_name).ok_or_else(|| {
                    de::Error::custom(format_args!("No registration found for {}", type_name))
                })?;
                let type_info = registration.type_info();
                let value = map.next_value_seed(TypedReflectDeserializer {
                    type_info,
                    registry: self.registry,
                })?;
                Ok(value)
            }
            Some(type_fields::TYPE) => Err(de::Error::duplicate_field(type_fields::TYPE)),
            Some(field) => Err(de::Error::unknown_field(field, &[type_fields::VALUE])),
            None => Err(de::Error::invalid_length(
                0,
                &"two entries: `type` and `value`",
            )),
        }
    }
}

/// A deserializer for reflected types whose [`TypeInfo`] is known.
///
/// For non-value types, this will return the dynamic equivalent. For example, a
/// deserialized struct will return a [`DynamicStruct`] and a `Vec` will return a
/// [`DynamicList`].
///
/// [`TypeInfo`]: crate::TypeInfo
/// [`DynamicStruct`]: crate::DynamicStruct
/// [`DynamicList`]: crate::DynamicList
pub struct TypedReflectDeserializer<'a> {
    type_info: &'a TypeInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for TypedReflectDeserializer<'a> {
    type Value = Box<dyn Reflect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Handle both Value case and types that have a custom `ReflectDeserialize`
        let type_id = self.type_info.type_id();
        let type_name = self.type_info.type_name();
        let registration = self.registry.get(type_id).ok_or_else(|| {
            de::Error::custom(format_args!("no registration found for {}", type_name))
        })?;

        if let Some(deserialize_reflect) = registration.data::<ReflectDeserialize>() {
            let value = deserialize_reflect.deserialize(deserializer)?;
            return Ok(value);
        }

        match self.type_info {
            TypeInfo::Struct(struct_info) => {
                let mut dynamic_struct = deserializer.deserialize_map(StructVisitor {
                    struct_info,
                    registry: self.registry,
                })?;
                dynamic_struct.set_name(struct_info.type_name().to_string());
                Ok(Box::new(dynamic_struct))
            }
            TypeInfo::TupleStruct(tuple_struct_info) => {
                let mut dynamic_tuple_struct = deserializer.deserialize_tuple(
                    tuple_struct_info.field_len(),
                    TupleStructVisitor {
                        tuple_struct_info,
                        registry: self.registry,
                    },
                )?;
                dynamic_tuple_struct.set_name(tuple_struct_info.type_name().to_string());
                Ok(Box::new(dynamic_tuple_struct))
            }
            TypeInfo::List(list_info) => {
                let mut dynamic_list = deserializer.deserialize_seq(ListVisitor {
                    list_info,
                    registry: self.registry,
                })?;
                dynamic_list.set_name(list_info.type_name().to_string());
                Ok(Box::new(dynamic_list))
            }
            TypeInfo::Array(array_info) => {
                let mut dynamic_array = deserializer.deserialize_tuple(
                    array_info.capacity(),
                    ArrayVisitor {
                        array_info,
                        registry: self.registry,
                    },
                )?;
                dynamic_array.set_name(array_info.type_name().to_string());
                Ok(Box::new(dynamic_array))
            }
            TypeInfo::Map(map_info) => {
                let mut dynamic_map = deserializer.deserialize_map(MapVisitor {
                    map_info,
                    registry: self.registry,
                })?;
                dynamic_map.set_name(map_info.type_name().to_string());
                Ok(Box::new(dynamic_map))
            }
            TypeInfo::Tuple(tuple_info) => {
                let mut dynamic_tuple = deserializer.deserialize_tuple(
                    tuple_info.field_len(),
                    TupleVisitor {
                        tuple_info,
                        registry: self.registry,
                    },
                )?;
                dynamic_tuple.set_name(tuple_info.type_name().to_string());
                Ok(Box::new(dynamic_tuple))
            }
            TypeInfo::Enum(enum_info) => {
                let mut dynamic_enum = deserializer.deserialize_map(EnumVisitor {
                    enum_info,
                    registry: self.registry,
                })?;
                dynamic_enum.set_name(enum_info.type_name().to_string());
                Ok(Box::new(dynamic_enum))
            }
            TypeInfo::Value(_) => {
                // This case should already be handled
                Err(de::Error::custom(format_args!(
                    "the TypeRegistration for {} doesn't have ReflectDeserialize",
                    type_name
                )))
            }
            TypeInfo::Dynamic(_) => {
                // We could potentially allow this but we'd have no idea what the actual types of the
                // fields are and would rely on the deserializer to determine them (e.g. `i32` vs `i64`)
                Err(de::Error::custom(format_args!(
                    "cannot deserialize arbitrary dynamic type {}",
                    type_name
                )))
            }
        }
    }
}

struct StructVisitor<'a> {
    struct_info: &'a StructInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for StructVisitor<'a> {
    type Value = DynamicStruct;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected struct value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        visit_struct(&mut map, self.struct_info, self.registry)
    }
}

struct TupleStructVisitor<'a> {
    tuple_struct_info: &'a TupleStructInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for TupleStructVisitor<'a> {
    type Value = DynamicTupleStruct;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected tuple struct value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut index = 0usize;
        let mut tuple_struct = DynamicTupleStruct::default();

        let get_field_info = |index: usize| -> Result<&'a TypeInfo, V::Error> {
            let field = self.tuple_struct_info.field_at(index).ok_or_else(|| {
                de::Error::custom(format_args!(
                    "no field at index {} on tuple {}",
                    index,
                    self.tuple_struct_info.type_name(),
                ))
            })?;
            get_type_info(field.type_id(), field.type_name(), self.registry)
        };

        while let Some(value) = seq.next_element_seed(TypedReflectDeserializer {
            type_info: get_field_info(index)?,
            registry: self.registry,
        })? {
            tuple_struct.insert_boxed(value);
            index += 1;
            if index >= self.tuple_struct_info.field_len() {
                break;
            }
        }

        if tuple_struct.field_len() != self.tuple_struct_info.field_len() {
            return Err(Error::invalid_length(
                tuple_struct.field_len(),
                &self.tuple_struct_info.field_len().to_string().as_str(),
            ));
        }

        Ok(tuple_struct)
    }
}

struct ListVisitor<'a> {
    list_info: &'a ListInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ListVisitor<'a> {
    type Value = DynamicList;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected list value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut list = DynamicList::default();
        let type_info = get_type_info(
            self.list_info.item_type_id(),
            self.list_info.item_type_name(),
            self.registry,
        )?;
        while let Some(value) = seq.next_element_seed(TypedReflectDeserializer {
            type_info,
            registry: self.registry,
        })? {
            list.push_box(value);
        }
        Ok(list)
    }
}

struct ArrayVisitor<'a> {
    array_info: &'a ArrayInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ArrayVisitor<'a> {
    type Value = DynamicArray;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected array value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        let type_info = get_type_info(
            self.array_info.item_type_id(),
            self.array_info.item_type_name(),
            self.registry,
        )?;
        while let Some(value) = seq.next_element_seed(TypedReflectDeserializer {
            type_info,
            registry: self.registry,
        })? {
            vec.push(value);
        }

        if vec.len() != self.array_info.capacity() {
            return Err(Error::invalid_length(
                vec.len(),
                &self.array_info.capacity().to_string().as_str(),
            ));
        }

        Ok(DynamicArray::new(vec.into_boxed_slice()))
    }
}

struct MapVisitor<'a> {
    map_info: &'a MapInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for MapVisitor<'a> {
    type Value = DynamicMap;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected map value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut dynamic_map = DynamicMap::default();
        let key_type_info = get_type_info(
            self.map_info.key_type_id(),
            self.map_info.key_type_name(),
            self.registry,
        )?;
        let value_type_info = get_type_info(
            self.map_info.value_type_id(),
            self.map_info.value_type_name(),
            self.registry,
        )?;
        while let Some(key) = map.next_key_seed(TypedReflectDeserializer {
            type_info: key_type_info,
            registry: self.registry,
        })? {
            let value = map.next_value_seed(TypedReflectDeserializer {
                type_info: value_type_info,
                registry: self.registry,
            })?;
            dynamic_map.insert_boxed(key, value);
        }

        Ok(dynamic_map)
    }
}

struct TupleVisitor<'a> {
    tuple_info: &'a TupleInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for TupleVisitor<'a> {
    type Value = DynamicTuple;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected tuple value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(&mut seq, self.tuple_info, self.registry)
    }
}

struct EnumVisitor<'a> {
    enum_info: &'a EnumInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for EnumVisitor<'a> {
    type Value = DynamicEnum;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected enum value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let variant_name = map
            .next_key::<&str>()?
            .ok_or_else(|| Error::missing_field("the variant name of the enum"))?;

        let variant_info = self
            .enum_info
            .variant(variant_name)
            .ok_or_else(|| Error::custom(format_args!("unknown variant {}", variant_name)))?;

        let mut dynamic_enum = DynamicEnum::default();

        match variant_info {
            VariantInfo::Struct(struct_info) => {
                let dynamic_struct = map.next_value_seed(StructVariantDeserializer {
                    struct_info,
                    registry: self.registry,
                })?;
                dynamic_enum.set_variant(variant_name, dynamic_struct);
            }
            VariantInfo::Tuple(tuple_info) => {
                let dynamic_tuple = map.next_value_seed(TupleVariantDeserializer {
                    tuple_info,
                    registry: self.registry,
                })?;
                dynamic_enum.set_variant(variant_name, dynamic_tuple);
            }
            VariantInfo::Unit(..) => {
                map.next_value::<()>()?;
                dynamic_enum.set_variant(variant_name, ());
            }
        }

        Ok(dynamic_enum)
    }
}

struct StructVariantDeserializer<'a> {
    struct_info: &'a StructVariantInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for StructVariantDeserializer<'a> {
    type Value = DynamicStruct;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(StructVariantVisitor {
            struct_info: self.struct_info,
            registry: self.registry,
        })
    }
}

struct StructVariantVisitor<'a> {
    struct_info: &'a StructVariantInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for StructVariantVisitor<'a> {
    type Value = DynamicStruct;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected struct variant value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        visit_struct(&mut map, self.struct_info, self.registry)
    }
}

struct TupleVariantDeserializer<'a> {
    tuple_info: &'a TupleVariantInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for TupleVariantDeserializer<'a> {
    type Value = DynamicTuple;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_tuple(
            self.tuple_info.field_len(),
            TupleVariantVisitor {
                tuple_info: self.tuple_info,
                registry: self.registry,
            },
        )
    }
}

struct TupleVariantVisitor<'a> {
    tuple_info: &'a TupleVariantInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for TupleVariantVisitor<'a> {
    type Value = DynamicTuple;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected tuple variant value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(&mut seq, self.tuple_info, self.registry)
    }
}

fn visit_struct<'de, T, V>(
    map: &mut V,
    info: &T,
    registry: &TypeRegistry,
) -> Result<DynamicStruct, V::Error>
where
    T: StructLikeInfo,
    V: MapAccess<'de>,
{
    let mut dynamic_struct = DynamicStruct::default();
    while let Some(key) = map.next_key::<String>()? {
        let field = info.get_field(&key).ok_or_else(|| {
            Error::custom(format_args!(
                "no field named {} on struct {}",
                key,
                info.get_name(),
            ))
        })?;
        let type_info = get_type_info(field.type_id(), field.type_name(), registry)?;
        let value = map.next_value_seed(TypedReflectDeserializer {
            type_info,
            registry,
        })?;
        dynamic_struct.insert_boxed(&key, value);
    }

    Ok(dynamic_struct)
}

fn visit_tuple<'de, T, V>(
    seq: &mut V,
    info: &T,
    registry: &TypeRegistry,
) -> Result<DynamicTuple, V::Error>
where
    T: TupleLikeInfo,
    V: SeqAccess<'de>,
{
    let mut tuple = DynamicTuple::default();
    let mut index = 0usize;

    let get_field_info = |index: usize| -> Result<&TypeInfo, V::Error> {
        let field = info.get_field(index).ok_or_else(|| {
            Error::custom(format_args!(
                "no field at index {} on tuple {}",
                index,
                info.get_name(),
            ))
        })?;
        get_type_info(field.type_id(), field.type_name(), registry)
    };

    while let Some(value) = seq.next_element_seed(TypedReflectDeserializer {
        type_info: get_field_info(index)?,
        registry,
    })? {
        tuple.insert_boxed(value);
        index += 1;
        if index >= info.get_field_len() {
            break;
        }
    }

    let len = info.get_field_len();

    if tuple.field_len() != len {
        return Err(Error::invalid_length(
            tuple.field_len(),
            &len.to_string().as_str(),
        ));
    }

    Ok(tuple)
}

fn get_type_info<'a, E: de::Error>(
    type_id: TypeId,
    type_name: &'a str,
    registry: &'a TypeRegistry,
) -> Result<&'a TypeInfo, E> {
    let registration = registry.get(type_id).ok_or_else(|| {
        de::Error::custom(format_args!("no registration found for type {}", type_name))
    })?;
    Ok(registration.type_info())
}

#[cfg(test)]
mod tests {
    use crate as bevy_reflect;
    use crate::serde::ReflectDeserializer;
    use crate::{DynamicEnum, FromReflect, Reflect, ReflectDeserialize, TypeRegistry};
    use bevy_utils::HashMap;
    use serde::de::DeserializeSeed;
    use serde::Deserialize;
    use std::f32::consts::PI;

    #[derive(Reflect, FromReflect, Debug, PartialEq)]
    struct MyStruct {
        primitive_value: i8,
        option_value: Option<String>,
        tuple_value: (f32, usize),
        list_value: Vec<i32>,
        array_value: [i32; 5],
        map_value: HashMap<u8, usize>,
        struct_value: SomeStruct,
        tuple_struct_value: SomeTupleStruct,
        custom_deserialize: CustomDeserialize,
    }

    #[derive(Reflect, FromReflect, Debug, PartialEq, Deserialize)]
    struct SomeStruct {
        foo: i64,
    }

    #[derive(Reflect, FromReflect, Debug, PartialEq)]
    struct SomeTupleStruct(String);

    /// Implements a custom deserialize using #[reflect(Deserialize)].
    ///
    /// For testing purposes, this is just the auto-generated one from deriving.
    #[derive(Reflect, FromReflect, Debug, PartialEq, Deserialize)]
    #[reflect(Deserialize)]
    struct CustomDeserialize {
        value: usize,
        #[serde(rename = "renamed")]
        inner_struct: SomeStruct,
    }

    fn get_registry() -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        registry.register::<MyStruct>();
        registry.register::<SomeStruct>();
        registry.register::<SomeTupleStruct>();
        registry.register::<CustomDeserialize>();
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
        registry.register::<Option<String>>();
        registry.register_type_data::<Option<String>, ReflectDeserialize>();
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
            "value": {
                "Unit": (),
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
            "value": {
                "NewType": (123),
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
            "value": {
                "Tuple": (1.23, 3.21),
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
            "value": {
                "Struct": {
                    "value": "I <3 Enums",
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

    #[test]
    fn should_deserialize() {
        let mut map = HashMap::new();
        map.insert(64, 32);

        let expected = MyStruct {
            primitive_value: 123,
            option_value: Some(String::from("Hello world!")),
            tuple_value: (PI, 1337),
            list_value: vec![-2, -1, 0, 1, 2],
            array_value: [-2, -1, 0, 1, 2],
            map_value: map,
            struct_value: SomeStruct { foo: 999999999 },
            tuple_struct_value: SomeTupleStruct(String::from("Tuple Struct")),
            custom_deserialize: CustomDeserialize {
                value: 100,
                inner_struct: SomeStruct { foo: 101 },
            },
        };

        let input = r#"{
            "type": "bevy_reflect::serde::de::tests::MyStruct",
            "value": {
                "primitive_value": 123,
                "option_value": Some("Hello world!"),
                "tuple_value": (
                    3.1415927,
                    1337,
                ),
                "list_value": [
                    -2,
                    -1,
                    0,
                    1,
                    2,
                ],
                "array_value": (
                    -2,
                    -1,
                    0,
                    1,
                    2,
                ),
                "map_value": {
                    64: 32,
                },
                "struct_value": {
                    "foo": 999999999,
                },
                "tuple_struct_value": ("Tuple Struct"),
                "custom_deserialize": (
                    value: 100,
                    renamed: (
                        foo: 101,
                    ),
                )
            },
        }"#;

        let registry = get_registry();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output = <MyStruct as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output);
    }
}
