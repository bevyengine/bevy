use crate::serde::TypedReflectSerializer;
use crate::{List, TypeRegistry};
use serde::ser::SerializeSeq;
use serde::Serialize;

/// A serializer for [`List`] values.
pub(super) struct ListSerializer<'a> {
    list: &'a dyn List,
    registry: &'a TypeRegistry,
}

impl<'a> ListSerializer<'a> {
    pub fn new(list: &'a dyn List, registry: &'a TypeRegistry) -> Self {
        Self { list, registry }
    }
}

impl<'a> Serialize for ListSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.list.len()))?;
        for value in self.list.iter() {
            state.serialize_element(&TypedReflectSerializer::new_internal(value, self.registry))?;
        }
        state.end()
    }
}
