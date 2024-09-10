use crate::serde::de::tuple_utils::visit_tuple;
use crate::{DynamicTuple, TupleInfo, TypeRegistration, TypeRegistry};
use core::fmt::Formatter;
use serde::de::{SeqAccess, Visitor};
use std::fmt;

/// A [`Visitor`] for deserializing [`Tuple`] values.
///
/// [`Tuple`]: crate::Tuple
pub(super) struct TupleVisitor<'a> {
    tuple_info: &'static TupleInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a> TupleVisitor<'a> {
    pub fn new(
        tuple_info: &'static TupleInfo,
        registration: &'a TypeRegistration,
        registry: &'a TypeRegistry,
    ) -> Self {
        Self {
            tuple_info,
            registration,
            registry,
        }
    }
}

impl<'a, 'de> Visitor<'de> for TupleVisitor<'a> {
    type Value = DynamicTuple;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected tuple value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(&mut seq, self.tuple_info, self.registration, self.registry)
    }
}
