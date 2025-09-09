use crate::{
    serde::{ser::error_utils::make_custom_error, SerializationData, TypedReflectSerializer},
    Struct, TypeInfo, TypeRegistry,
};
use serde::{ser::SerializeStruct, Serialize};

use super::ReflectSerializerProcessor;

/// A serializer for [`Struct`] values.
pub(super) struct StructSerializer<'a, P> {
    pub struct_value: &'a dyn Struct,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a P>,
}

impl<P: ReflectSerializerProcessor> Serialize for StructSerializer<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let type_info = self
            .struct_value
            .get_represented_type_info()
            .ok_or_else(|| {
                make_custom_error(format_args!(
                    "cannot get type info for `{}`",
                    self.struct_value.reflect_type_path()
                ))
            })?;

        let struct_info = match type_info {
            TypeInfo::Struct(struct_info) => struct_info,
            info => {
                return Err(make_custom_error(format_args!(
                    "expected struct type but received {info:?}"
                )));
            }
        };

        let serialization_data = self
            .registry
            .get(type_info.type_id())
            .and_then(|registration| registration.data::<SerializationData>());
        let ignored_len = serialization_data.map(SerializationData::len).unwrap_or(0);
        let mut state = serializer.serialize_struct(
            struct_info.type_path_table().ident().unwrap(),
            self.struct_value.field_len() - ignored_len,
        )?;

        for (index, value) in self.struct_value.iter_fields().enumerate() {
            if serialization_data.is_some_and(|data| data.is_field_skipped(index)) {
                continue;
            }
            let key = struct_info.field_at(index).unwrap().name();
            state.serialize_field(
                key,
                &TypedReflectSerializer::new_internal(value, self.registry, self.processor),
            )?;
        }
        state.end()
    }
}
