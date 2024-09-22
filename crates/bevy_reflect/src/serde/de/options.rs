use crate::serde::de::error_utils::make_custom_error;
use crate::serde::de::registration_utils::try_get_registration;
use crate::serde::TypedReflectDeserializer;
use crate::{DynamicEnum, DynamicTuple, EnumInfo, TypeRegistry, VariantInfo};
use core::fmt::Formatter;
use serde::de::{DeserializeSeed, Error, Visitor};
use std::fmt;

/// A [`Visitor`] for deserializing [`Option`] values.
pub(super) struct OptionVisitor<'a> {
    enum_info: &'static EnumInfo,
    registry: &'a TypeRegistry,
}

impl<'a> OptionVisitor<'a> {
    pub fn new(enum_info: &'static EnumInfo, registry: &'a TypeRegistry) -> Self {
        Self {
            enum_info,
            registry,
        }
    }
}

impl<'a, 'de> Visitor<'de> for OptionVisitor<'a> {
    type Value = DynamicEnum;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected option value of type ")?;
        formatter.write_str(self.enum_info.type_path())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut option = DynamicEnum::default();
        option.set_variant("None", ());
        Ok(option)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let variant_info = self.enum_info.variant("Some").unwrap();
        match variant_info {
            VariantInfo::Tuple(tuple_info) if tuple_info.field_len() == 1 => {
                let field = tuple_info.field_at(0).unwrap();
                let registration = try_get_registration(*field.ty(), self.registry)?;
                let de = TypedReflectDeserializer::new_internal(registration, self.registry);
                let mut value = DynamicTuple::default();
                value.insert_boxed(de.deserialize(deserializer)?);
                let mut option = DynamicEnum::default();
                option.set_variant("Some", value);
                Ok(option)
            }
            info => Err(make_custom_error(format_args!(
                "invalid variant, expected `Some` but got `{}`",
                info.name()
            ))),
        }
    }
}
