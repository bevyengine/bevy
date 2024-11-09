use crate::{
    serde::{de::registration_utils::try_get_registration, TypedReflectDeserializer},
    DynamicSet, Set, SetInfo, TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{SeqAccess, Visitor};

/// A [`Visitor`] for deserializing [`Set`] values.
///
/// [`Set`]: crate::Set
pub(super) struct SetVisitor<'a> {
    set_info: &'static SetInfo,
    registry: &'a TypeRegistry,
}

impl<'a> SetVisitor<'a> {
    pub fn new(set_info: &'static SetInfo, registry: &'a TypeRegistry) -> Self {
        Self { set_info, registry }
    }
}

impl<'a, 'de> Visitor<'de> for SetVisitor<'a> {
    type Value = DynamicSet;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected set value")
    }

    fn visit_seq<V>(self, mut set: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut dynamic_set = DynamicSet::default();
        let value_registration = try_get_registration(self.set_info.value_ty(), self.registry)?;
        while let Some(value) = set.next_element_seed(TypedReflectDeserializer::new_internal(
            value_registration,
            self.registry,
        ))? {
            dynamic_set.insert_boxed(value);
        }

        Ok(dynamic_set)
    }
}
