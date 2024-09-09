use crate::serde::ser::error_utils::make_custom_error;
use crate::serde::TypedReflectSerializer;
use crate::{Tuple, TypeRegistry};
use serde::ser::SerializeTuple;
use serde::Serialize;

/// A serializer for [`Tuple`] values.
pub(super) struct TupleSerializer<'a> {
    tuple: &'a dyn Tuple,
    registry: &'a TypeRegistry,
}

impl<'a> TupleSerializer<'a> {
    pub fn new(tuple: &'a dyn Tuple, registry: &'a TypeRegistry) -> Self {
        Self { tuple, registry }
    }
}

impl<'a> Serialize for TupleSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let type_info = self.tuple.get_represented_type_info().ok_or_else(|| {
            make_custom_error(format_args!(
                "cannot get type info for `{}`",
                self.tuple.reflect_type_path()
            ))
        })?;

        let tuple_info = type_info.as_tuple().map_err(make_custom_error)?;

        let mut state = serializer.serialize_tuple(self.tuple.field_len())?;

        for (index, value) in self.tuple.iter_fields().enumerate() {
            let info = tuple_info.field_at(index).unwrap().type_info();

            state.serialize_element(&TypedReflectSerializer::new_internal(
                value,
                info,
                self.registry,
            ))?;
        }
        state.end()
    }
}
