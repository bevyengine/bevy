use crate::{
    serde::{
        de::{
            error_utils::make_custom_error,
            helpers::{ExpectedValues, Ident},
            registration_utils::try_get_registration,
        },
        SerializationData, TypedReflectDeserializer,
    },
    DynamicStruct, NamedField, StructInfo, StructVariantInfo, TypeRegistration, TypeRegistry,
};
use alloc::string::ToString;
use core::slice::Iter;
use serde::de::{Error, MapAccess, SeqAccess};

use super::ReflectDeserializerProcessor;

/// A helper trait for accessing type information from struct-like types.
pub(super) trait StructLikeInfo {
    fn field<E: Error>(&self, name: &str) -> Result<&NamedField, E>;
    fn field_at<E: Error>(&self, index: usize) -> Result<&NamedField, E>;
    fn field_len(&self) -> usize;
    fn iter_fields(&self) -> Iter<'_, NamedField>;
}

impl StructLikeInfo for StructInfo {
    fn field<E: Error>(&self, name: &str) -> Result<&NamedField, E> {
        Self::field(self, name).ok_or_else(|| {
            make_custom_error(format_args!(
                "no field named `{}` on struct `{}`",
                name,
                self.type_path(),
            ))
        })
    }

    fn field_at<E: Error>(&self, index: usize) -> Result<&NamedField, E> {
        Self::field_at(self, index).ok_or_else(|| {
            make_custom_error(format_args!(
                "no field at index `{}` on struct `{}`",
                index,
                self.type_path(),
            ))
        })
    }

    fn field_len(&self) -> usize {
        Self::field_len(self)
    }

    fn iter_fields(&self) -> Iter<'_, NamedField> {
        self.iter()
    }
}

impl StructLikeInfo for StructVariantInfo {
    fn field<E: Error>(&self, name: &str) -> Result<&NamedField, E> {
        Self::field(self, name).ok_or_else(|| {
            make_custom_error(format_args!(
                "no field named `{}` on variant `{}`",
                name,
                self.name(),
            ))
        })
    }

    fn field_at<E: Error>(&self, index: usize) -> Result<&NamedField, E> {
        Self::field_at(self, index).ok_or_else(|| {
            make_custom_error(format_args!(
                "no field at index `{}` on variant `{}`",
                index,
                self.name(),
            ))
        })
    }

    fn field_len(&self) -> usize {
        Self::field_len(self)
    }

    fn iter_fields(&self) -> Iter<'_, NamedField> {
        self.iter()
    }
}

/// Deserializes a [struct-like] type from a mapping of fields, returning a [`DynamicStruct`].
///
/// [struct-like]: StructLikeInfo
pub(super) fn visit_struct<'de, T, V, P>(
    map: &mut V,
    info: &'static T,
    registration: &TypeRegistration,
    registry: &TypeRegistry,
    mut processor: Option<&mut P>,
) -> Result<DynamicStruct, V::Error>
where
    T: StructLikeInfo,
    V: MapAccess<'de>,
    P: ReflectDeserializerProcessor,
{
    let mut dynamic_struct = DynamicStruct::default();
    while let Some(Ident(key)) = map.next_key::<Ident>()? {
        let field = info.field::<V::Error>(&key).map_err(|_| {
            let fields = info.iter_fields().map(NamedField::name);
            make_custom_error(format_args!(
                "unknown field `{}`, expected one of {:?}",
                key,
                ExpectedValues::from_iter(fields)
            ))
        })?;
        let registration = try_get_registration(*field.ty(), registry)?;
        let value = map.next_value_seed(TypedReflectDeserializer::new_internal(
            registration,
            registry,
            processor.as_deref_mut(),
        ))?;
        dynamic_struct.insert_boxed(&key, value);
    }

    if let Some(serialization_data) = registration.data::<SerializationData>() {
        for (skipped_index, skipped_field) in serialization_data.iter_skipped() {
            let Ok(field) = info.field_at::<V::Error>(*skipped_index) else {
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
pub(super) fn visit_struct_seq<'de, T, V, P>(
    seq: &mut V,
    info: &T,
    registration: &TypeRegistration,
    registry: &TypeRegistry,
    mut processor: Option<&mut P>,
) -> Result<DynamicStruct, V::Error>
where
    T: StructLikeInfo,
    V: SeqAccess<'de>,
    P: ReflectDeserializerProcessor,
{
    let mut dynamic_struct = DynamicStruct::default();

    let len = info.field_len();

    if len == 0 {
        // Handle unit structs
        return Ok(dynamic_struct);
    }

    let serialization_data = registration.data::<SerializationData>();

    for index in 0..len {
        let name = info.field_at::<V::Error>(index)?.name();

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
            .next_element_seed(TypedReflectDeserializer::new_internal(
                try_get_registration(*info.field_at(index)?.ty(), registry)?,
                registry,
                processor.as_deref_mut(),
            ))?
            .ok_or_else(|| Error::invalid_length(index, &len.to_string().as_str()))?;
        dynamic_struct.insert_boxed(name, value);
    }

    Ok(dynamic_struct)
}
