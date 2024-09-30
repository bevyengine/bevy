use crate::{
    serde::de::struct_utils::{visit_struct, visit_struct_seq},
    DynamicStruct, StructInfo, TypeRegistration, TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{MapAccess, SeqAccess, Visitor};

use super::ReflectDeserializerProcessor;

/// A [`Visitor`] for deserializing [`Struct`] values.
///
/// [`Struct`]: crate::Struct
pub(super) struct StructVisitor<'a, P> {
    pub struct_info: &'static StructInfo,
    pub registration: &'a TypeRegistration,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a mut P>,
}

impl<'de, P: ReflectDeserializerProcessor> Visitor<'de> for StructVisitor<'_, P> {
    type Value = DynamicStruct;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected struct value")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        visit_struct_seq(
            &mut seq,
            self.struct_info,
            self.registration,
            self.registry,
            self.processor,
        )
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        visit_struct(
            &mut map,
            self.struct_info,
            self.registration,
            self.registry,
            self.processor,
        )
    }
}
