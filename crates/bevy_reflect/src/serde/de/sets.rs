use crate::{
    serde::{de::registration_utils::try_get_registration, TypedReflectDeserializer},
    DynamicSet, Set, SetInfo, TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{SeqAccess, Visitor};

use super::ReflectDeserializerProcessor;

/// A [`Visitor`] for deserializing [`Set`] values.
///
/// [`Set`]: crate::Set
pub(super) struct SetVisitor<'a, P> {
    pub set_info: &'static SetInfo,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a mut P>,
}

impl<'de, P: ReflectDeserializerProcessor> Visitor<'de> for SetVisitor<'_, P> {
    type Value = DynamicSet;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected set value")
    }

    fn visit_seq<V>(mut self, mut set: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut dynamic_set = DynamicSet::default();
        let value_registration = try_get_registration(self.set_info.value_ty(), self.registry)?;
        while let Some(value) = set.next_element_seed(TypedReflectDeserializer::new_internal(
            value_registration,
            self.registry,
            self.processor.as_deref_mut(),
        ))? {
            dynamic_set.insert_boxed(value);
        }

        Ok(dynamic_set)
    }
}
