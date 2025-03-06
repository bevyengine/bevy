use crate::{
    serde::{de::registration_utils::try_get_registration, TypedReflectDeserializer},
    DynamicList, ListInfo, TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{SeqAccess, Visitor};

use super::ReflectDeserializerProcessor;

/// A [`Visitor`] for deserializing [`List`] values.
///
/// [`List`]: crate::List
pub(super) struct ListVisitor<'a, P> {
    pub list_info: &'static ListInfo,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a mut P>,
}

impl<'de, P: ReflectDeserializerProcessor> Visitor<'de> for ListVisitor<'_, P> {
    type Value = DynamicList;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected list value")
    }

    fn visit_seq<V>(mut self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut list = DynamicList::default();
        let registration = try_get_registration(self.list_info.item_ty(), self.registry)?;
        while let Some(value) = seq.next_element_seed(TypedReflectDeserializer::new_internal(
            registration,
            self.registry,
            self.processor.as_deref_mut(),
        ))? {
            list.push_box(value);
        }
        Ok(list)
    }
}
