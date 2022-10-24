use crate::serde::SerializationData;
use crate::{
    ArrayInfo, DynamicArray, DynamicEnum, DynamicList, DynamicMap, DynamicStruct, DynamicTuple,
    DynamicTupleStruct, DynamicVariant, EnumInfo, ListInfo, Map, MapInfo, NamedField, Reflect,
    ReflectDeserialize, StructInfo, StructVariantInfo, Tuple, TupleInfo, TupleStruct,
    TupleStructInfo, TupleVariantInfo, TypeInfo, TypeRegistration, TypeRegistry, UnnamedField,
    VariantInfo,
};
use erased_serde::Deserializer;
use serde::de::{
    self, DeserializeSeed, EnumAccess, Error, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::Deserialize;
use std::any::TypeId;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::slice::Iter;

pub trait DeserializeValue {
    fn deserialize(
        deserializer: &mut dyn Deserializer,
        type_registry: &TypeRegistry,
    ) -> Result<Box<dyn Reflect>, erased_serde::Error>;
}

trait StructLikeInfo {
    fn get_name(&self) -> &str;
    fn get_field(&self, name: &str) -> Option<&NamedField>;
    fn iter_fields(&self) -> Iter<'_, NamedField>;
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

    fn iter_fields(&self) -> Iter<'_, NamedField> {
        self.iter()
    }
}

impl StructLikeInfo for StructVariantInfo {
    fn get_name(&self) -> &str {
        self.name()
    }

    fn get_field(&self, name: &str) -> Option<&NamedField> {
        self.field(name)
    }

    fn iter_fields(&self) -> Iter<'_, NamedField> {
        self.iter()
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

/// A debug struct used for error messages that displays a list of expected values.
///
/// # Example
///
/// ```ignore
/// let expected = vec!["foo", "bar", "baz"];
/// assert_eq!("`foo`, `bar`, `baz`", format!("{}", ExpectedValues(expected)));
/// ```
struct ExpectedValues<T: Display>(Vec<T>);

impl<T: Display> Debug for ExpectedValues<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let len = self.0.len();
        for (index, item) in self.0.iter().enumerate() {
            write!(f, "`{}`", item)?;
            if index < len - 1 {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

/// Represents a simple reflected identifier.
#[derive(Debug, Clone, Eq, PartialEq)]
struct Ident(String);

impl<'de> Deserialize<'de> for Ident {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct IdentVisitor;

        impl<'de> Visitor<'de> for IdentVisitor {
            type Value = Ident;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("identifier")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Ident(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Ident(value))
            }
        }

        deserializer.deserialize_identifier(IdentVisitor)
    }
}

/// A general purpose deserializer for reflected types.
///
/// This will return a [`Box<dyn Reflect>`] containing the deserialized data.
/// For non-value types, this `Box` will contain the dynamic equivalent. For example, a
/// deserialized struct will return a [`DynamicStruct`] and a `Vec` will return a
/// [`DynamicList`]. For value types, this `Box` will contain the actual value.
/// For example, an `f32` will contain the actual `f32` type.
///
/// This means that converting to any concrete instance will require the use of
/// [`FromReflect`], or downcasting for value types.
///
/// Because the type isn't known ahead of time, the serialized data must take the form of
/// a map containing the following entries (in order):
/// 1. `type`: The _full_ [type name]
/// 2. `value`: The serialized value of the reflected type
///
/// If the type is already known and the [`TypeInfo`] for it can be retrieved,
/// [`TypedReflectDeserializer`] may be used instead to avoid requiring these entries.
///
/// [`Box<dyn Reflect>`]: crate::Reflect
/// [`DynamicStruct`]: crate::DynamicStruct
/// [`DynamicList`]: crate::DynamicList
/// [`FromReflect`]: crate::FromReflect
/// [type name]: std::any::type_name
pub struct UntypedReflectDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a> UntypedReflectDeserializer<'a> {
    pub fn new(registry: &'a TypeRegistry) -> Self {
        Self { registry }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for UntypedReflectDeserializer<'a> {
    type Value = Box<dyn Reflect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(UntypedReflectDeserializerVisitor {
            registry: self.registry,
        })
    }
}

struct UntypedReflectDeserializerVisitor<'a> {
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for UntypedReflectDeserializerVisitor<'a> {
    type Value = Box<dyn Reflect>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("map containing `type` and `value` entries for the reflected value")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let type_name = map
            .next_key::<String>()?
            .ok_or_else(|| Error::invalid_length(0, &"at least one entry"))?;

        let registration = self.registry.get_with_name(&type_name).ok_or_else(|| {
            Error::custom(format_args!("No registration found for `{}`", type_name))
        })?;
        let value = map.next_value_seed(TypedReflectDeserializer {
            registration,
            registry: self.registry,
        })?;
        Ok(value)
    }
}

/// A deserializer for reflected types whose [`TypeInfo`] is known.
///
/// This will return a [`Box<dyn Reflect>`] containing the deserialized data.
/// For non-value types, this `Box` will contain the dynamic equivalent. For example, a
/// deserialized struct will return a [`DynamicStruct`] and a `Vec` will return a
/// [`DynamicList`]. For value types, this `Box` will contain the actual value.
/// For example, an `f32` will contain the actual `f32` type.
///
/// This means that converting to any concrete instance will require the use of
/// [`FromReflect`], or downcasting for value types.
///
/// If the type is not known ahead of time, use [`UntypedReflectDeserializer`] instead.
///
/// [`TypeInfo`]: crate::TypeInfo
/// [`Box<dyn Reflect>`]: crate::Reflect
/// [`DynamicStruct`]: crate::DynamicStruct
/// [`DynamicList`]: crate::DynamicList
/// [`FromReflect`]: crate::FromReflect
pub struct TypedReflectDeserializer<'a> {
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a> TypedReflectDeserializer<'a> {
    pub fn new(registration: &'a TypeRegistration, registry: &'a TypeRegistry) -> Self {
        Self {
            registration,
            registry,
        }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for TypedReflectDeserializer<'a> {
    type Value = Box<dyn Reflect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let type_name = self.registration.type_name();

        // Handle both Value case and types that have a custom `ReflectDeserialize`
        if let Some(deserialize_reflect) = self.registration.data::<ReflectDeserialize>() {
            let value = deserialize_reflect.deserialize(deserializer)?;
            return Ok(value);
        }

        match self.registration.type_info() {
            TypeInfo::Struct(struct_info) => {
                let mut dynamic_struct = deserializer.deserialize_struct(
                    struct_info.name(),
                    // Field names are mainly just a hint, we don't necessarily need to store and pass that data
                    &[],
                    StructVisitor {
                        struct_info,
                        registry: self.registry,
                    },
                )?;
                dynamic_struct.set_name(struct_info.type_name().to_string());
                Ok(Box::new(dynamic_struct))
            }
            TypeInfo::TupleStruct(tuple_struct_info) => {
                let mut dynamic_tuple_struct = deserializer.deserialize_tuple_struct(
                    tuple_struct_info.name(),
                    tuple_struct_info.field_len(),
                    TupleStructVisitor {
                        tuple_struct_info,
                        registry: self.registry,
                        registration: self.registration,
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
                let type_name = enum_info.type_name();
                let mut dynamic_enum = if type_name.starts_with("core::option::Option") {
                    deserializer.deserialize_option(OptionVisitor {
                        enum_info,
                        registry: self.registry,
                    })?
                } else {
                    deserializer.deserialize_enum(
                        enum_info.name(),
                        // Variant names are mainly just a hint, we don't necessarily need to store and pass that data
                        &[],
                        EnumVisitor {
                            enum_info,
                            registry: self.registry,
                        },
                    )?
                };
                dynamic_enum.set_name(type_name.to_string());
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
    struct_info: &'static StructInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for StructVisitor<'a> {
    type Value = DynamicStruct;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
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
    tuple_struct_info: &'static TupleStructInfo,
    registry: &'a TypeRegistry,
    registration: &'a TypeRegistration,
}

impl<'a, 'de> Visitor<'de> for TupleStructVisitor<'a> {
    type Value = DynamicTupleStruct;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("reflected tuple struct value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut index = 0usize;
        let mut tuple_struct = DynamicTupleStruct::default();

        let get_field_registration = |index: usize| -> Result<&'a TypeRegistration, V::Error> {
            let field = self.tuple_struct_info.field_at(index).ok_or_else(|| {
                de::Error::custom(format_args!(
                    "no field at index {} on tuple {}",
                    index,
                    self.tuple_struct_info.type_name(),
                ))
            })?;
            get_registration(field.type_id(), field.type_name(), self.registry)
        };

        while let Some(value) = seq.next_element_seed(TypedReflectDeserializer {
            registration: get_field_registration(index)?,
            registry: self.registry,
        })? {
            tuple_struct.insert_boxed(value);
            index += 1;
            if index >= self.tuple_struct_info.field_len() {
                break;
            }
        }

        let ignored_len = self
            .registration
            .data::<SerializationData>()
            .map(|data| data.len())
            .unwrap_or(0);
        if tuple_struct.field_len() != self.tuple_struct_info.field_len() - ignored_len {
            return Err(Error::invalid_length(
                tuple_struct.field_len(),
                &self.tuple_struct_info.field_len().to_string().as_str(),
            ));
        }

        Ok(tuple_struct)
    }
}

struct TupleVisitor<'a> {
    tuple_info: &'static TupleInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for TupleVisitor<'a> {
    type Value = DynamicTuple;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("reflected tuple value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(&mut seq, self.tuple_info, self.registry)
    }
}

struct ArrayVisitor<'a> {
    array_info: &'static ArrayInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ArrayVisitor<'a> {
    type Value = DynamicArray;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("reflected array value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        let registration = get_registration(
            self.array_info.item_type_id(),
            self.array_info.item_type_name(),
            self.registry,
        )?;
        while let Some(value) = seq.next_element_seed(TypedReflectDeserializer {
            registration,
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

struct ListVisitor<'a> {
    list_info: &'static ListInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ListVisitor<'a> {
    type Value = DynamicList;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("reflected list value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut list = DynamicList::default();
        let registration = get_registration(
            self.list_info.item_type_id(),
            self.list_info.item_type_name(),
            self.registry,
        )?;
        while let Some(value) = seq.next_element_seed(TypedReflectDeserializer {
            registration,
            registry: self.registry,
        })? {
            list.push_box(value);
        }
        Ok(list)
    }
}

struct MapVisitor<'a> {
    map_info: &'static MapInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for MapVisitor<'a> {
    type Value = DynamicMap;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("reflected map value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut dynamic_map = DynamicMap::default();
        let key_registration = get_registration(
            self.map_info.key_type_id(),
            self.map_info.key_type_name(),
            self.registry,
        )?;
        let value_registration = get_registration(
            self.map_info.value_type_id(),
            self.map_info.value_type_name(),
            self.registry,
        )?;
        while let Some(key) = map.next_key_seed(TypedReflectDeserializer {
            registration: key_registration,
            registry: self.registry,
        })? {
            let value = map.next_value_seed(TypedReflectDeserializer {
                registration: value_registration,
                registry: self.registry,
            })?;
            dynamic_map.insert_boxed(key, value);
        }

        Ok(dynamic_map)
    }
}

struct EnumVisitor<'a> {
    enum_info: &'static EnumInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for EnumVisitor<'a> {
    type Value = DynamicEnum;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("reflected enum value")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let mut dynamic_enum = DynamicEnum::default();
        let (Ident(variant_name), variant) = data.variant().unwrap();
        let variant_info = self.enum_info.variant(&variant_name).ok_or_else(|| {
            let names = self.enum_info.iter().map(|variant| variant.name());
            Error::custom(format_args!(
                "unknown variant `{}`, expected one of {:?}",
                variant_name,
                ExpectedValues(names.collect())
            ))
        })?;
        let value: DynamicVariant = match variant_info {
            VariantInfo::Unit(..) => variant.unit_variant()?.into(),
            VariantInfo::Struct(struct_info) => variant
                .struct_variant(
                    // Field names are mainly just a hint, we don't necessarily need to store and pass that data
                    &[],
                    StructVariantVisitor {
                        struct_info,
                        registry: self.registry,
                    },
                )?
                .into(),
            VariantInfo::Tuple(tuple_info) if tuple_info.field_len() == 1 => {
                let field = tuple_info.field_at(0).unwrap();
                let registration =
                    get_registration(field.type_id(), field.type_name(), self.registry)?;
                let value = variant.newtype_variant_seed(TypedReflectDeserializer {
                    registration,
                    registry: self.registry,
                })?;
                let mut dynamic_tuple = DynamicTuple::default();
                dynamic_tuple.insert_boxed(value);
                dynamic_tuple.into()
            }
            VariantInfo::Tuple(tuple_info) => variant
                .tuple_variant(
                    tuple_info.field_len(),
                    TupleVariantVisitor {
                        tuple_info,
                        registry: self.registry,
                    },
                )?
                .into(),
        };

        dynamic_enum.set_variant(variant_name, value);
        Ok(dynamic_enum)
    }
}

struct StructVariantVisitor<'a> {
    struct_info: &'static StructVariantInfo,
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

struct TupleVariantVisitor<'a> {
    tuple_info: &'static TupleVariantInfo,
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

struct OptionVisitor<'a> {
    enum_info: &'static EnumInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for OptionVisitor<'a> {
    type Value = DynamicEnum;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected option value of type ")?;
        formatter.write_str(self.enum_info.type_name())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let variant_info = self.enum_info.variant("Some").unwrap();
        match variant_info {
            VariantInfo::Tuple(tuple_info) if tuple_info.field_len() == 1 => {
                let field = tuple_info.field_at(0).unwrap();
                let registration =
                    get_registration(field.type_id(), field.type_name(), self.registry)?;
                let de = TypedReflectDeserializer {
                    registration,
                    registry: self.registry,
                };
                let mut value = DynamicTuple::default();
                value.insert_boxed(de.deserialize(deserializer)?);
                let mut option = DynamicEnum::default();
                option.set_variant("Some", value);
                Ok(option)
            }
            info => Err(Error::custom(format_args!(
                "invalid variant, expected `Some` but got `{}`",
                info.name()
            ))),
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut option = DynamicEnum::default();
        option.set_variant("None", ());
        Ok(option)
    }
}

fn visit_struct<'de, T, V>(
    map: &mut V,
    info: &'static T,
    registry: &TypeRegistry,
) -> Result<DynamicStruct, V::Error>
where
    T: StructLikeInfo,
    V: MapAccess<'de>,
{
    let mut dynamic_struct = DynamicStruct::default();
    while let Some(Ident(key)) = map.next_key::<Ident>()? {
        let field = info.get_field(&key).ok_or_else(|| {
            let fields = info.iter_fields().map(|field| field.name());
            Error::custom(format_args!(
                "unknown field `{}`, expected one of {:?}",
                key,
                ExpectedValues(fields.collect())
            ))
        })?;
        let registration = get_registration(field.type_id(), field.type_name(), registry)?;
        let value = map.next_value_seed(TypedReflectDeserializer {
            registration,
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

    let get_field_registration = |index: usize| -> Result<&TypeRegistration, V::Error> {
        let field = info.get_field(index).ok_or_else(|| {
            Error::invalid_length(index, &info.get_field_len().to_string().as_str())
        })?;
        get_registration(field.type_id(), field.type_name(), registry)
    };

    while let Some(value) = seq.next_element_seed(TypedReflectDeserializer {
        registration: get_field_registration(index)?,
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

fn get_registration<'a, E: Error>(
    type_id: TypeId,
    type_name: &str,
    registry: &'a TypeRegistry,
) -> Result<&'a TypeRegistration, E> {
    let registration = registry.get(type_id).ok_or_else(|| {
        Error::custom(format_args!(
            "no registration found for type `{}`",
            type_name
        ))
    })?;
    Ok(registration)
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::f32::consts::PI;

    use serde::de::DeserializeSeed;
    use serde::Deserialize;

    use bevy_utils::HashMap;

    use crate as bevy_reflect;
    use crate::serde::{TypedReflectDeserializer, UntypedReflectDeserializer};
    use crate::{DynamicEnum, FromReflect, Reflect, ReflectDeserialize, TypeRegistry};

    #[derive(Reflect, FromReflect, Debug, PartialEq)]
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
        unit_enum: SomeEnum,
        newtype_enum: SomeEnum,
        tuple_enum: SomeEnum,
        struct_enum: SomeEnum,
        custom_deserialize: CustomDeserialize,
    }

    #[derive(Reflect, FromReflect, Debug, PartialEq)]
    struct SomeStruct {
        foo: i64,
    }

    #[derive(Reflect, FromReflect, Debug, PartialEq)]
    struct SomeTupleStruct(String);

    #[derive(Reflect, FromReflect, Debug, PartialEq, Deserialize)]
    struct SomeDeserializableStruct {
        foo: i64,
    }

    /// Implements a custom deserialize using `#[reflect(Deserialize)]`.
    ///
    /// For testing purposes, this is just the auto-generated one from deriving.
    #[derive(Reflect, FromReflect, Debug, PartialEq, Deserialize)]
    #[reflect(Deserialize)]
    struct CustomDeserialize {
        value: usize,
        #[serde(rename = "renamed")]
        inner_struct: SomeDeserializableStruct,
    }

    #[derive(Reflect, FromReflect, Debug, PartialEq)]
    enum SomeEnum {
        Unit,
        NewType(usize),
        Tuple(f32, f32),
        Struct { foo: String },
    }

    fn get_registry() -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        registry.register::<MyStruct>();
        registry.register::<SomeStruct>();
        registry.register::<SomeTupleStruct>();
        registry.register::<CustomDeserialize>();
        registry.register::<SomeDeserializableStruct>();
        registry.register::<SomeEnum>();
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
        registry.register::<Option<SomeStruct>>();
        registry.register::<Option<String>>();
        registry.register_type_data::<Option<String>, ReflectDeserialize>();
        registry
    }

    #[test]
    fn should_deserialize() {
        let mut map = HashMap::new();
        map.insert(64, 32);

        let expected = MyStruct {
            primitive_value: 123,
            option_value: Some(String::from("Hello world!")),
            option_value_complex: Some(SomeStruct { foo: 123 }),
            tuple_value: (PI, 1337),
            list_value: vec![-2, -1, 0, 1, 2],
            array_value: [-2, -1, 0, 1, 2],
            map_value: map,
            struct_value: SomeStruct { foo: 999999999 },
            tuple_struct_value: SomeTupleStruct(String::from("Tuple Struct")),
            unit_enum: SomeEnum::Unit,
            newtype_enum: SomeEnum::NewType(123),
            tuple_enum: SomeEnum::Tuple(1.23, 3.21),
            struct_enum: SomeEnum::Struct {
                foo: String::from("Struct variant value"),
            },
            custom_deserialize: CustomDeserialize {
                value: 100,
                inner_struct: SomeDeserializableStruct { foo: 101 },
            },
        };

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
                struct_value: (
                    foo: 999999999,
                ),
                tuple_struct_value: ("Tuple Struct"),
                unit_enum: Unit,
                newtype_enum: NewType(123),
                tuple_enum: Tuple(1.23, 3.21),
                struct_enum: Struct(
                    foo: "Struct variant value",
                ),
                custom_deserialize: (
                    value: 100,
                    renamed: (
                        foo: 101,
                    ),
                ),
            ),
        }"#;

        let registry = get_registry();
        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
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
        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();
        let output = dynamic_output
            .take::<f32>()
            .expect("underlying type should be f32");
        assert_eq!(1.23, output);
    }

    #[test]
    fn should_deserialized_typed() {
        #[derive(Reflect, FromReflect, Debug, PartialEq)]
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

        let output = <Foo as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_deserialize_option() {
        #[derive(Reflect, FromReflect, Debug, PartialEq)]
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
            "bevy_reflect::serde::de::tests::should_deserialize_option::OptionTest": (
                none: None,
                simple: Some("Hello world!"),
                complex: Some((
                    foo: 123,
                )),
            ),
        }"#;

        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
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
            "bevy_reflect::serde::de::tests::should_deserialize_option::OptionTest": (
                none: None,
                simple: "Hello world!",
                complex: (
                    foo: 123,
                ),
            ),
        }"#;

        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
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
    "bevy_reflect::serde::de::tests::enum_should_deserialize::MyEnum": Unit,
}"#;
        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Unit);
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === NewType Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::enum_should_deserialize::MyEnum": NewType(123),
}"#;
        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::NewType(123));
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === Tuple Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::enum_should_deserialize::MyEnum": Tuple(1.23, 3.21),
}"#;
        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Tuple(1.23, 3.21));
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === Struct Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::enum_should_deserialize::MyEnum": Struct(
        value: "I <3 Enums",
    ),
}"#;
        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Struct {
            value: String::from("I <3 Enums"),
        });
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());
    }
}
