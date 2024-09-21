use crate::serde::de::error_utils::make_custom_error;
use crate::serde::de::registration_utils::try_get_registration;
use crate::serde::{SerializationData, TypedReflectDeserializer};
use crate::{
    DynamicTuple, TupleInfo, TupleStructInfo, TupleVariantInfo, TypeRegistration, TypeRegistry,
    UnnamedField,
};
use serde::de::{Error, SeqAccess};

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
pub(super) fn visit_tuple<'de, T, V>(
    seq: &mut V,
    info: &T,
    registration: &TypeRegistration,
    registry: &TypeRegistry,
) -> Result<DynamicTuple, V::Error>
where
    T: TupleLikeInfo,
    V: SeqAccess<'de>,
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
            ))?
            .ok_or_else(|| Error::invalid_length(index, &len.to_string().as_str()))?;
        tuple.insert_boxed(value);
    }

    Ok(tuple)
}
