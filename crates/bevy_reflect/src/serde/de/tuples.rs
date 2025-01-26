use crate::{
    serde::de::tuple_utils::visit_tuple, DynamicTuple, TupleInfo, TypeRegistration, TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{SeqAccess, Visitor};

use super::ReflectDeserializerProcessor;

/// A [`Visitor`] for deserializing [`Tuple`] values.
///
/// [`Tuple`]: crate::Tuple
pub(super) struct TupleVisitor<'a, P> {
    pub tuple_info: &'static TupleInfo,
    pub registration: &'a TypeRegistration,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a mut P>,
}

impl<'de, P: ReflectDeserializerProcessor> Visitor<'de> for TupleVisitor<'_, P> {
    type Value = DynamicTuple;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected tuple value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(
            &mut seq,
            self.tuple_info,
            self.registration,
            self.registry,
            self.processor,
        )
    }
}
