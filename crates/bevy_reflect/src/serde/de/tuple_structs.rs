use crate::{
    serde::{de::tuple_utils::visit_tuple, SerializationData},
    DynamicTupleStruct, TupleStructInfo, TypeRegistration, TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{DeserializeSeed, SeqAccess, Visitor};

use super::{registration_utils::try_get_registration, TypedReflectDeserializer};

use super::ReflectDeserializerProcessor;

/// A [`Visitor`] for deserializing [`TupleStruct`] values.
///
/// [`TupleStruct`]: crate::TupleStruct
pub(super) struct TupleStructVisitor<'a, P> {
    pub tuple_struct_info: &'static TupleStructInfo,
    pub registration: &'a TypeRegistration,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a mut P>,
}

impl<'de, P: ReflectDeserializerProcessor> Visitor<'de> for TupleStructVisitor<'_, P> {
    type Value = DynamicTupleStruct;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected tuple struct value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(
            &mut seq,
            self.tuple_struct_info,
            self.registration,
            self.registry,
            self.processor,
        )
        .map(DynamicTupleStruct::from)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut tuple = DynamicTupleStruct::default();
        let serialization_data = self.registration.data::<SerializationData>();

        if let Some(value) = serialization_data.and_then(|data| data.generate_default(0)) {
            tuple.insert_boxed(value.into_partial_reflect());
            return Ok(tuple);
        }

        let registration = try_get_registration(
            *self
                .tuple_struct_info
                .field_at(0)
                .ok_or(serde::de::Error::custom("Field at index 0 not found"))?
                .ty(),
            self.registry,
        )?;
        let reflect_deserializer =
            TypedReflectDeserializer::new_internal(registration, self.registry, self.processor);
        let value = reflect_deserializer.deserialize(deserializer)?;

        tuple.insert_boxed(value.into_partial_reflect());

        Ok(tuple)
    }
}
