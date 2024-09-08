use crate::serde::de::registration_utils::GetFieldRegistration;
use crate::serde::{SerializationData, TypedReflectDeserializer};
use crate::{
    DynamicTuple, TupleInfo, TupleStructInfo, TupleVariantInfo, TypeRegistration, TypeRegistry,
};
use serde::de::{Error, SeqAccess};

pub(super) trait TupleLikeInfo {
    fn get_field_len(&self) -> usize;
}

impl TupleLikeInfo for TupleInfo {
    fn get_field_len(&self) -> usize {
        self.field_len()
    }
}

impl TupleLikeInfo for TupleStructInfo {
    fn get_field_len(&self) -> usize {
        self.field_len()
    }
}

impl TupleLikeInfo for TupleVariantInfo {
    fn get_field_len(&self) -> usize {
        self.field_len()
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
    T: TupleLikeInfo + GetFieldRegistration,
    V: SeqAccess<'de>,
{
    let mut tuple = DynamicTuple::default();

    let len = info.get_field_len();

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
            .next_element_seed(TypedReflectDeserializer::new(
                info.get_field_registration(index, registry)?,
                registry,
            ))?
            .ok_or_else(|| Error::invalid_length(index, &len.to_string().as_str()))?;
        tuple.insert_boxed(value);
    }

    Ok(tuple)
}
