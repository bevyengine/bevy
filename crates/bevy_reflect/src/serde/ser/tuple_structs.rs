use crate::{
    serde::{ser::error_utils::make_custom_error, SerializationData, TypedReflectSerializer},
    TupleStruct, TypeInfo, TypeRegistry,
};
use serde::{ser::SerializeTupleStruct, Serialize};

/// A serializer for [`TupleStruct`] values.
pub(super) struct TupleStructSerializer<'a> {
    tuple_struct: &'a dyn TupleStruct,
    registry: &'a TypeRegistry,
}

impl<'a> TupleStructSerializer<'a> {
    pub fn new(tuple_struct: &'a dyn TupleStruct, registry: &'a TypeRegistry) -> Self {
        Self {
            tuple_struct,
            registry,
        }
    }
}

impl<'a> Serialize for TupleStructSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let type_info = self
            .tuple_struct
            .get_represented_type_info()
            .ok_or_else(|| {
                make_custom_error(format_args!(
                    "cannot get type info for `{}`",
                    self.tuple_struct.reflect_type_path()
                ))
            })?;

        let tuple_struct_info = match type_info {
            TypeInfo::TupleStruct(tuple_struct_info) => tuple_struct_info,
            info => {
                return Err(make_custom_error(format_args!(
                    "expected tuple struct type but received {info:?}"
                )));
            }
        };

        let serialization_data = self
            .registry
            .get(type_info.type_id())
            .and_then(|registration| registration.data::<SerializationData>());
        let ignored_len = serialization_data.map(SerializationData::len).unwrap_or(0);
        let mut state = serializer.serialize_tuple_struct(
            tuple_struct_info.type_path_table().ident().unwrap(),
            self.tuple_struct.field_len() - ignored_len,
        )?;

        for (index, value) in self.tuple_struct.iter_fields().enumerate() {
            if serialization_data
                .map(|data| data.is_field_skipped(index))
                .unwrap_or(false)
            {
                continue;
            }
            state.serialize_field(&TypedReflectSerializer::new_internal(value, self.registry))?;
        }
        state.end()
    }
}
