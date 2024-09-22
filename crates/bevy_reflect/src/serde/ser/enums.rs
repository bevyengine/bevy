use crate::serde::ser::error_utils::make_custom_error;
use crate::serde::TypedReflectSerializer;
use crate::{Enum, TypeRegistry, VariantType};
use serde::ser::{SerializeStructVariant, SerializeTupleVariant};
use serde::Serialize;

/// A serializer for [`Enum`] values.
pub(super) struct EnumSerializer<'a> {
    enum_value: &'a dyn Enum,
    registry: &'a TypeRegistry,
}

impl<'a> EnumSerializer<'a> {
    pub fn new(enum_value: &'a dyn Enum, registry: &'a TypeRegistry) -> Self {
        Self {
            enum_value,
            registry,
        }
    }
}

impl<'a> Serialize for EnumSerializer<'a> {
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

        let enum_info = type_info.as_enum().map_err(make_custom_error)?;

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
                let struct_info = variant_info
                    .as_struct_variant()
                    .map_err(make_custom_error)?;

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
                            field_info.type_info(),
                            self.registry,
                        ),
                    )?;
                }
                state.end()
            }
            VariantType::Tuple if field_len == 1 => {
                let variant_info = variant_info.as_tuple_variant().map_err(make_custom_error)?;
                let info = variant_info.field_at(0).unwrap().type_info();

                let field = self.enum_value.field_at(0).unwrap();

                if type_info.type_path_table().module_path() == Some("core::option")
                    && type_info.type_path_table().ident() == Some("Option")
                {
                    serializer.serialize_some(&TypedReflectSerializer::new_internal(
                        field,
                        info,
                        self.registry,
                    ))
                } else {
                    serializer.serialize_newtype_variant(
                        enum_name,
                        variant_index,
                        variant_name,
                        &TypedReflectSerializer::new_internal(field, info, self.registry),
                    )
                }
            }
            VariantType::Tuple => {
                let variant_info = variant_info.as_tuple_variant().map_err(make_custom_error)?;

                let mut state = serializer.serialize_tuple_variant(
                    enum_name,
                    variant_index,
                    variant_name,
                    field_len,
                )?;
                for (index, field) in self.enum_value.iter_fields().enumerate() {
                    let info = variant_info.field_at(index).unwrap().type_info();

                    state.serialize_field(&TypedReflectSerializer::new_internal(
                        field.value(),
                        info,
                        self.registry,
                    ))?;
                }
                state.end()
            }
        }
    }
}
