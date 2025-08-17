use crate::{
    serde::{
        de::{error_utils::make_custom_error, registration_utils::try_get_registration},
        TypedReflectDeserializer,
    },
    DynamicEnum, DynamicTuple, EnumInfo, TypeRegistry, VariantInfo,
};
use core::{fmt, fmt::Formatter};
use serde::de::{DeserializeSeed, Error, Visitor};

use super::ReflectDeserializerProcessor;

/// A [`Visitor`] for deserializing [`Option`] values.
pub(super) struct OptionVisitor<'a, P> {
    pub enum_info: &'static EnumInfo,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a mut P>,
}

impl<'de, P: ReflectDeserializerProcessor> Visitor<'de> for OptionVisitor<'_, P> {
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
                let de = TypedReflectDeserializer::new_internal(
                    registration,
                    self.registry,
                    self.processor,
                );
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
