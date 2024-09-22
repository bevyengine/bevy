use crate::serde::ser::error_utils::make_custom_error;
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
        let type_info = self.set.get_represented_type_info().ok_or_else(|| {
            make_custom_error(format_args!(
                "cannot get type info for `{}`",
                self.set.reflect_type_path()
            ))
        })?;

        let set_info = type_info.as_set().map_err(make_custom_error)?;
        let value_info = set_info.value_info();

        let mut state = serializer.serialize_seq(Some(self.set.len()))?;
        for value in self.set.iter() {
            state.serialize_element(&TypedReflectSerializer::new_internal(
                value,
                value_info,
                self.registry,
            ))?;
        }
        state.end()
    }
}
