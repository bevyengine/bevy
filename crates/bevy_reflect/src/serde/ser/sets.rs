use crate::serde::TypedReflectSerializer;
use crate::{Set, TypeRegistry};
use serde::ser::SerializeSeq;
use serde::Serialize;

/// A serializer for [`Set`] values.
pub(super) struct SetSerializer<'a> {
    set: &'a dyn Set,
    registry: &'a TypeRegistry,
}

impl<'a> SetSerializer<'a> {
    pub fn new(set: &'a dyn Set, registry: &'a TypeRegistry) -> Self {
        Self { set, registry }
    }
}

impl<'a> Serialize for SetSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.set.len()))?;
        for value in self.set.iter() {
            state.serialize_element(&TypedReflectSerializer::new_internal(value, self.registry))?;
        }
        state.end()
    }
}
