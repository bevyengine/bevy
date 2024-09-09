use crate::serde::ser::error_utils::make_custom_error;
use crate::serde::TypedReflectSerializer;
use crate::{Array, TypeRegistry};
use serde::ser::SerializeTuple;
use serde::Serialize;

/// A serializer for [`Array`] values.
pub(super) struct ArraySerializer<'a> {
    array: &'a dyn Array,
    registry: &'a TypeRegistry,
}

impl<'a> ArraySerializer<'a> {
    pub fn new(array: &'a dyn Array, registry: &'a TypeRegistry) -> Self {
        Self { array, registry }
    }
}

impl<'a> Serialize for ArraySerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let type_info = self.array.get_represented_type_info().ok_or_else(|| {
            make_custom_error(format_args!(
                "cannot get type info for `{}`",
                self.array.reflect_type_path()
            ))
        })?;

        let array_info = type_info.as_array().map_err(make_custom_error)?;
        let item_info = array_info.item_info();

        let mut state = serializer.serialize_tuple(self.array.len())?;
        for value in self.array.iter() {
            state.serialize_element(&TypedReflectSerializer::new_internal(
                value,
                item_info,
                self.registry,
            ))?;
        }
        state.end()
    }
}
