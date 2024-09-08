use crate::serde::de::struct_utils::{visit_struct, visit_struct_seq};
use crate::{DynamicStruct, StructInfo, TypeRegistration, TypeRegistry};
use core::fmt::Formatter;
use serde::de::{MapAccess, SeqAccess, Visitor};
use std::fmt;

/// A [`Visitor`] for deserializing [`Struct`] values.
///
/// [`Struct`]: crate::Struct
pub(super) struct StructVisitor<'a> {
    struct_info: &'static StructInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a> StructVisitor<'a> {
    pub fn new(
        struct_info: &'static StructInfo,
        registration: &'a TypeRegistration,
        registry: &'a TypeRegistry,
    ) -> Self {
        Self {
            struct_info,
            registration,
            registry,
        }
    }
}

impl<'a, 'de> Visitor<'de> for StructVisitor<'a> {
    type Value = DynamicStruct;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected struct value")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        visit_struct_seq(&mut seq, self.struct_info, self.registration, self.registry)
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        visit_struct(&mut map, self.struct_info, self.registration, self.registry)
    }
}
