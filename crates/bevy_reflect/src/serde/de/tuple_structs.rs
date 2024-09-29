use crate::{
    serde::de::tuple_utils::visit_tuple, DynamicTupleStruct, TupleStructInfo, TypeRegistration,
    TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{SeqAccess, Visitor};

use super::ReflectDeserializerProcessor;

/// A [`Visitor`] for deserializing [`TupleStruct`] values.
///
/// [`TupleStruct`]: crate::TupleStruct
pub(super) struct TupleStructVisitor<'a, P> {
    pub tuple_struct_info: &'static TupleStructInfo,
    pub registration: &'a TypeRegistration,
    pub registry: &'a TypeRegistry,
    pub processor: P,
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
}
