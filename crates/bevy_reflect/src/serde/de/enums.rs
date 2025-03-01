use crate::{
    serde::{
        de::{
            error_utils::make_custom_error,
            helpers::ExpectedValues,
            registration_utils::try_get_registration,
            struct_utils::{visit_struct, visit_struct_seq},
            tuple_utils::{visit_tuple, TupleLikeInfo},
        },
        TypedReflectDeserializer,
    },
    DynamicEnum, DynamicStruct, DynamicTuple, DynamicVariant, EnumInfo, StructVariantInfo,
    TupleVariantInfo, TypeRegistration, TypeRegistry, VariantInfo,
};
use core::{fmt, fmt::Formatter};
use serde::de::{DeserializeSeed, EnumAccess, Error, MapAccess, SeqAccess, VariantAccess, Visitor};

use super::ReflectDeserializerProcessor;

/// A [`Visitor`] for deserializing [`Enum`] values.
///
/// [`Enum`]: crate::Enum
pub(super) struct EnumVisitor<'a, P> {
    pub enum_info: &'static EnumInfo,
    pub registration: &'a TypeRegistration,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a mut P>,
}

impl<'de, P: ReflectDeserializerProcessor> Visitor<'de> for EnumVisitor<'_, P> {
    type Value = DynamicEnum;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected enum value")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let mut dynamic_enum = DynamicEnum::default();
        let (variant_info, variant) = data.variant_seed(VariantDeserializer {
            enum_info: self.enum_info,
        })?;

        let value: DynamicVariant = match variant_info {
            VariantInfo::Unit(..) => variant.unit_variant()?.into(),
            VariantInfo::Struct(struct_info) => variant
                .struct_variant(
                    struct_info.field_names(),
                    StructVariantVisitor {
                        struct_info,
                        registration: self.registration,
                        registry: self.registry,
                        processor: self.processor,
                    },
                )?
                .into(),
            VariantInfo::Tuple(tuple_info) if tuple_info.field_len() == 1 => {
                let registration = try_get_registration(
                    *TupleLikeInfo::field_at(tuple_info, 0)?.ty(),
                    self.registry,
                )?;
                let value =
                    variant.newtype_variant_seed(TypedReflectDeserializer::new_internal(
                        registration,
                        self.registry,
                        self.processor,
                    ))?;
                let mut dynamic_tuple = DynamicTuple::default();
                dynamic_tuple.insert_boxed(value);
                dynamic_tuple.into()
            }
            VariantInfo::Tuple(tuple_info) => variant
                .tuple_variant(
                    tuple_info.field_len(),
                    TupleVariantVisitor {
                        tuple_info,
                        registration: self.registration,
                        registry: self.registry,
                        processor: self.processor,
                    },
                )?
                .into(),
        };
        let variant_name = variant_info.name();
        let variant_index = self
            .enum_info
            .index_of(variant_name)
            .expect("variant should exist");
        dynamic_enum.set_variant_with_index(variant_index, variant_name, value);
        Ok(dynamic_enum)
    }
}

struct VariantDeserializer {
    enum_info: &'static EnumInfo,
}

impl<'de> DeserializeSeed<'de> for VariantDeserializer {
    type Value = &'static VariantInfo;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VariantVisitor(&'static EnumInfo);

        impl<'de> Visitor<'de> for VariantVisitor {
            type Value = &'static VariantInfo;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("expected either a variant index or variant name")
            }

            fn visit_u32<E>(self, variant_index: u32) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.0.variant_at(variant_index as usize).ok_or_else(|| {
                    make_custom_error(format_args!(
                        "no variant found at index `{}` on enum `{}`",
                        variant_index,
                        self.0.type_path()
                    ))
                })
            }

            fn visit_str<E>(self, variant_name: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.0.variant(variant_name).ok_or_else(|| {
                    let names = self.0.iter().map(VariantInfo::name);
                    make_custom_error(format_args!(
                        "unknown variant `{}`, expected one of {:?}",
                        variant_name,
                        ExpectedValues::from_iter(names)
                    ))
                })
            }
        }

        deserializer.deserialize_identifier(VariantVisitor(self.enum_info))
    }
}

struct StructVariantVisitor<'a, P> {
    struct_info: &'static StructVariantInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
    processor: Option<&'a mut P>,
}

impl<'de, P: ReflectDeserializerProcessor> Visitor<'de> for StructVariantVisitor<'_, P> {
    type Value = DynamicStruct;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected struct variant value")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        visit_struct_seq(
            &mut seq,
            self.struct_info,
            self.registration,
            self.registry,
            self.processor,
        )
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        visit_struct(
            &mut map,
            self.struct_info,
            self.registration,
            self.registry,
            self.processor,
        )
    }
}

struct TupleVariantVisitor<'a, P> {
    tuple_info: &'static TupleVariantInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
    processor: Option<&'a mut P>,
}

impl<'de, P: ReflectDeserializerProcessor> Visitor<'de> for TupleVariantVisitor<'_, P> {
    type Value = DynamicTuple;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected tuple variant value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(
            &mut seq,
            self.tuple_info,
            self.registration,
            self.registry,
            self.processor,
        )
    }
}
