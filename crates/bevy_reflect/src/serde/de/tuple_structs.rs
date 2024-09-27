use crate::{
    serde::de::tuple_utils::visit_tuple, DynamicTupleStruct, TupleStructInfo, TypeRegistration,
    TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{SeqAccess, Visitor};

/// A [`Visitor`] for deserializing [`TupleStruct`] values.
///
/// [`TupleStruct`]: crate::TupleStruct
pub(super) struct TupleStructVisitor<'a> {
    tuple_struct_info: &'static TupleStructInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a> TupleStructVisitor<'a> {
    pub fn new(
        tuple_struct_info: &'static TupleStructInfo,
        registration: &'a TypeRegistration,
        registry: &'a TypeRegistry,
    ) -> Self {
        Self {
            tuple_struct_info,
            registration,
            registry,
        }
    }
}

impl<'a, 'de> Visitor<'de> for TupleStructVisitor<'a> {
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
        )
        .map(DynamicTupleStruct::from)
    }
}
