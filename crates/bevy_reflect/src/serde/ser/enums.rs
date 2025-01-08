use crate::{
    serde::{ser::error_utils::make_custom_error, TypedReflectSerializer},
    Enum, TypeInfo, TypeRegistry, VariantInfo, VariantType,
};
use serde::{
    ser::{SerializeStructVariant, SerializeTupleVariant},
    Serialize,
};

use super::ReflectSerializerProcessor;

/// A serializer for [`Enum`] values.
pub(super) struct EnumSerializer<'a, P> {
    pub enum_value: &'a dyn Enum,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a P>,
}

impl<P: ReflectSerializerProcessor> Serialize for EnumSerializer<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let type_info = self.enum_value.get_represented_type_info().ok_or_else(|| {
            make_custom_error(format_args!(
                "cannot get type info for `{}`",
                self.enum_value.reflect_type_path()
            ))
        })?;

        let enum_info = match type_info {
            TypeInfo::Enum(enum_info) => enum_info,
            info => {
                return Err(make_custom_error(format_args!(
                    "expected enum type but received {info:?}"
                )));
            }
        };

        let enum_name = enum_info.type_path_table().ident().unwrap();
        let variant_index = self.enum_value.variant_index() as u32;
        let variant_info = enum_info
            .variant_at(variant_index as usize)
            .ok_or_else(|| {
                make_custom_error(format_args!(
                    "variant at index `{variant_index}` does not exist",
                ))
            })?;
        let variant_name = variant_info.name();
        let variant_type = self.enum_value.variant_type();
        let field_len = self.enum_value.field_len();

        match variant_type {
            VariantType::Unit => {
                if type_info.type_path_table().module_path() == Some("core::option")
                    && type_info.type_path_table().ident() == Some("Option")
                {
                    serializer.serialize_none()
                } else {
                    serializer.serialize_unit_variant(enum_name, variant_index, variant_name)
                }
            }
            VariantType::Struct => {
                let struct_info = match variant_info {
                    VariantInfo::Struct(struct_info) => struct_info,
                    info => {
                        return Err(make_custom_error(format_args!(
                            "expected struct variant type but received {info:?}",
                        )));
                    }
                };

                let mut state = serializer.serialize_struct_variant(
                    enum_name,
                    variant_index,
                    variant_name,
                    field_len,
                )?;
                for (index, field) in self.enum_value.iter_fields().enumerate() {
                    let field_info = struct_info.field_at(index).unwrap();
                    state.serialize_field(
                        field_info.name(),
                        &TypedReflectSerializer::new_internal(
                            field.value(),
                            self.registry,
                            self.processor,
                        ),
                    )?;
                }
                state.end()
            }
            VariantType::Tuple if field_len == 1 => {
                let field = self.enum_value.field_at(0).unwrap();

                if type_info.type_path_table().module_path() == Some("core::option")
                    && type_info.type_path_table().ident() == Some("Option")
                {
                    serializer.serialize_some(&TypedReflectSerializer::new_internal(
                        field,
                        self.registry,
                        self.processor,
                    ))
                } else {
                    serializer.serialize_newtype_variant(
                        enum_name,
                        variant_index,
                        variant_name,
                        &TypedReflectSerializer::new_internal(field, self.registry, self.processor),
                    )
                }
            }
            VariantType::Tuple => {
                let mut state = serializer.serialize_tuple_variant(
                    enum_name,
                    variant_index,
                    variant_name,
                    field_len,
                )?;
                for field in self.enum_value.iter_fields() {
                    state.serialize_field(&TypedReflectSerializer::new_internal(
                        field.value(),
                        self.registry,
                        self.processor,
                    ))?;
                }
                state.end()
            }
        }
    }
}
