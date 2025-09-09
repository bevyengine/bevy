use crate::{
    serde::{
        de::{error_utils::make_custom_error, registration_utils::try_get_registration},
        SerializationData, TypedReflectDeserializer,
    },
    DynamicTuple, TupleInfo, TupleStructInfo, TupleVariantInfo, TypeRegistration, TypeRegistry,
    UnnamedField,
};
use alloc::string::ToString;
use serde::de::{Error, SeqAccess};

use super::ReflectDeserializerProcessor;

pub(super) trait TupleLikeInfo {
    fn field_at<E: Error>(&self, index: usize) -> Result<&UnnamedField, E>;
    fn field_len(&self) -> usize;
}

impl TupleLikeInfo for TupleInfo {
    fn field_len(&self) -> usize {
        Self::field_len(self)
    }

    fn field_at<E: Error>(&self, index: usize) -> Result<&UnnamedField, E> {
        Self::field_at(self, index).ok_or_else(|| {
            make_custom_error(format_args!(
                "no field at index `{}` on tuple `{}`",
                index,
                self.type_path(),
            ))
        })
    }
}

impl TupleLikeInfo for TupleStructInfo {
    fn field_len(&self) -> usize {
        Self::field_len(self)
    }

    fn field_at<E: Error>(&self, index: usize) -> Result<&UnnamedField, E> {
        Self::field_at(self, index).ok_or_else(|| {
            make_custom_error(format_args!(
                "no field at index `{}` on tuple struct `{}`",
                index,
                self.type_path(),
            ))
        })
    }
}

impl TupleLikeInfo for TupleVariantInfo {
    fn field_len(&self) -> usize {
        Self::field_len(self)
    }

    fn field_at<E: Error>(&self, index: usize) -> Result<&UnnamedField, E> {
        Self::field_at(self, index).ok_or_else(|| {
            make_custom_error(format_args!(
                "no field at index `{}` on tuple variant `{}`",
                index,
                self.name(),
            ))
        })
    }
}

/// Deserializes a [tuple-like] type from a sequence of elements, returning a [`DynamicTuple`].
///
/// [tuple-like]: TupleLikeInfo
pub(super) fn visit_tuple<'de, T, V, P>(
    seq: &mut V,
    info: &T,
    registration: &TypeRegistration,
    registry: &TypeRegistry,
    mut processor: Option<&mut P>,
) -> Result<DynamicTuple, V::Error>
where
    T: TupleLikeInfo,
    V: SeqAccess<'de>,
    P: ReflectDeserializerProcessor,
{
    let mut tuple = DynamicTuple::default();

    let len = info.field_len();

    if len == 0 {
        // Handle empty tuple/tuple struct
        return Ok(tuple);
    }

    let serialization_data = registration.data::<SerializationData>();

    for index in 0..len {
        if let Some(value) = serialization_data.and_then(|data| data.generate_default(index)) {
            tuple.insert_boxed(value.into_partial_reflect());
            continue;
        }

        let value = seq
            .next_element_seed(TypedReflectDeserializer::new_internal(
                try_get_registration(*info.field_at(index)?.ty(), registry)?,
                registry,
                processor.as_deref_mut(),
            ))?
            .ok_or_else(|| Error::invalid_length(index, &len.to_string().as_str()))?;
        tuple.insert_boxed(value);
    }

    Ok(tuple)
}
