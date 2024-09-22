use crate::serde::de::registration_utils::try_get_registration_data;
use crate::serde::TypedReflectDeserializer;
use crate::{DynamicList, ListInfo, TypeRegistry};
use core::fmt::Formatter;
use serde::de::{SeqAccess, Visitor};
use std::fmt;

/// A [`Visitor`] for deserializing [`List`] values.
///
/// [`List`]: crate::List
pub(super) struct ListVisitor<'a> {
    list_info: &'static ListInfo,
    registry: &'a TypeRegistry,
}

impl<'a> ListVisitor<'a> {
    pub fn new(list_info: &'static ListInfo, registry: &'a TypeRegistry) -> Self {
        Self {
            list_info,
            registry,
        }
    }
}

impl<'a, 'de> Visitor<'de> for ListVisitor<'a> {
    type Value = DynamicList;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected list value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut list = DynamicList::default();
        let data = try_get_registration_data(
            self.list_info.item_ty(),
            self.list_info.item_info(),
            self.registry,
        )?;
        while let Some(value) =
            seq.next_element_seed(TypedReflectDeserializer::new_internal(data, self.registry))?
        {
            list.push_box(value);
        }
        Ok(list)
    }
}
