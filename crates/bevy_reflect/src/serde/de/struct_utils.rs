use crate::serde::de::helpers::{ExpectedValues, Ident};
use crate::serde::de::registration_utils::{try_get_registration, GetFieldRegistration};
use crate::serde::{SerializationData, TypedReflectDeserializer};
use crate::{
    DynamicStruct, NamedField, StructInfo, StructVariantInfo, TypeRegistration, TypeRegistry,
};
use core::slice::Iter;
use serde::de::{Error, MapAccess, SeqAccess};

/// A helper trait for accessing type information from struct-like types.
pub(super) trait StructLikeInfo {
    fn get_field(&self, name: &str) -> Option<&NamedField>;
    fn field_at(&self, index: usize) -> Option<&NamedField>;
    fn get_field_len(&self) -> usize;
    fn iter_fields(&self) -> Iter<'_, NamedField>;
}

impl StructLikeInfo for StructInfo {
    fn get_field(&self, name: &str) -> Option<&NamedField> {
        self.field(name)
    }

    fn field_at(&self, index: usize) -> Option<&NamedField> {
        self.field_at(index)
    }

    fn get_field_len(&self) -> usize {
        self.field_len()
    }

    fn iter_fields(&self) -> Iter<'_, NamedField> {
        self.iter()
    }
}

impl StructLikeInfo for StructVariantInfo {
    fn get_field(&self, name: &str) -> Option<&NamedField> {
        self.field(name)
    }

    fn field_at(&self, index: usize) -> Option<&NamedField> {
        self.field_at(index)
    }

    fn get_field_len(&self) -> usize {
        self.field_len()
    }

    fn iter_fields(&self) -> Iter<'_, NamedField> {
        self.iter()
    }
}

/// Deserializes a [struct-like] type from a mapping of fields, returning a [`DynamicStruct`].
///
/// [struct-like]: StructLikeInfo
pub(super) fn visit_struct<'de, T, V>(
    map: &mut V,
    info: &'static T,
    registration: &TypeRegistration,
    registry: &TypeRegistry,
) -> Result<DynamicStruct, V::Error>
where
    T: StructLikeInfo,
    V: MapAccess<'de>,
{
    let mut dynamic_struct = DynamicStruct::default();
    while let Some(Ident(key)) = map.next_key::<Ident>()? {
        let field = info.get_field(&key).ok_or_else(|| {
            let fields = info.iter_fields().map(NamedField::name);
            Error::custom(format_args!(
                "unknown field `{}`, expected one of {:?}",
                key,
                ExpectedValues::from_iter(fields)
            ))
        })?;
        let registration = try_get_registration(*field.ty(), registry)?;
        let value = map.next_value_seed(TypedReflectDeserializer::new(registration, registry))?;
        dynamic_struct.insert_boxed(&key, value);
    }

    if let Some(serialization_data) = registration.data::<SerializationData>() {
        for (skipped_index, skipped_field) in serialization_data.iter_skipped() {
            let Some(field) = info.field_at(*skipped_index) else {
                continue;
            };
            dynamic_struct.insert_boxed(
                field.name(),
                skipped_field.generate_default().into_partial_reflect(),
            );
        }
    }

    Ok(dynamic_struct)
}

/// Deserializes a [struct-like] type from a sequence of fields, returning a [`DynamicStruct`].
///
/// [struct-like]: StructLikeInfo
pub(super) fn visit_struct_seq<'de, T, V>(
    seq: &mut V,
    info: &T,
    registration: &TypeRegistration,
    registry: &TypeRegistry,
) -> Result<DynamicStruct, V::Error>
where
    T: StructLikeInfo + GetFieldRegistration,
    V: SeqAccess<'de>,
{
    let mut dynamic_struct = DynamicStruct::default();

    let len = info.get_field_len();

    if len == 0 {
        // Handle unit structs
        return Ok(dynamic_struct);
    }

    let serialization_data = registration.data::<SerializationData>();

    for index in 0..len {
        let name = info.field_at(index).unwrap().name();

        if serialization_data
            .map(|data| data.is_field_skipped(index))
            .unwrap_or_default()
        {
            if let Some(value) = serialization_data.unwrap().generate_default(index) {
                dynamic_struct.insert_boxed(name, value.into_partial_reflect());
            }
            continue;
        }

        let value = seq
            .next_element_seed(TypedReflectDeserializer::new(
                info.get_field_registration(index, registry)?,
                registry,
            ))?
            .ok_or_else(|| Error::invalid_length(index, &len.to_string().as_str()))?;
        dynamic_struct.insert_boxed(name, value);
    }

    Ok(dynamic_struct)
}
