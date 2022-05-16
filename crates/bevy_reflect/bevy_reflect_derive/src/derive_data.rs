use quote::quote;
use crate::container_attributes::ReflectTraits;
use crate::field_attributes::{parse_field_attrs, ReflectFieldAttr};

use crate::{utility, REFLECT_ATTRIBUTE_NAME, REFLECT_VALUE_ATTRIBUTE_NAME};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Field, Fields, Generics, Ident, Meta, Path, Token, Variant};

pub(crate) enum ReflectDerive<'a> {
    Struct(ReflectStruct<'a>),
    TupleStruct(ReflectStruct<'a>),
    UnitStruct(ReflectStruct<'a>),
    Enum(ReflectEnum<'a>),
    Value(ReflectMeta<'a>),
}

/// Metadata present on all reflected types, including name, generics, and attributes.
///
/// # Example
///
/// ```ignore
/// #[derive(Reflect)]
/// //                          traits
/// //        |----------------------------------------|
/// #[reflect(PartialEq, Serialize, Deserialize, Default)]
/// //            type_name       generics
/// //     |-------------------||----------|
/// struct ThingThatImReflecting<T1, T2, T3> {/* ... */}
/// ```
pub(crate) struct ReflectMeta<'a> {
    /// The registered traits for this type.
    traits: ReflectTraits,
    /// The name of this type.
    type_name: &'a Ident,
    /// The generics defined on this type.
    generics: &'a Generics,
    /// A cached instance of the path to the `bevy_reflect` crate.
    bevy_reflect_path: Path,
}

/// Struct data used by derive macros for `Reflect` and `FromReflect`.
///
/// # Example
///
/// ```ignore
/// #[derive(Reflect)]
/// #[reflect(PartialEq, Serialize, Deserialize, Default)]
/// struct ThingThatImReflecting<T1, T2, T3> {
///     x: T1, // |
///     y: T2, // |- fields
///     z: T3  // |
/// }
/// ```
pub(crate) struct ReflectStruct<'a> {
    meta: ReflectMeta<'a>,
    fields: Vec<StructField<'a>>,
}

/// Enum data used by derive macros for `Reflect` and `FromReflect`.
///
/// # Example
///
/// ```ignore
/// #[derive(Reflect)]
/// #[reflect(PartialEq, Serialize, Deserialize, Default)]
/// enum ThingThatImReflecting<T1, T2, T3> {
///     A(T1),                  // |
///     B,                      // |- variants
///     C { foo: T2, bar: T3 }  // |
/// }
/// ```
pub(crate) struct ReflectEnum<'a> {
    meta: ReflectMeta<'a>,
    variants: Vec<EnumVariant<'a>>,
}

/// Represents a field on a struct or tuple struct.
pub(crate) struct StructField<'a> {
    /// The raw field.
    pub data: &'a Field,
    /// The reflection-based attributes on the field.
    pub attrs: ReflectFieldAttr,
    /// The index of this field within the struct.
    pub index: usize,
}

/// Represents a variant on an enum.
pub(crate) struct EnumVariant<'a> {
    /// The raw variant.
    pub data: &'a Variant,
    /// The fields within this variant.
    pub fields: EnumVariantFields<'a>,
    /// The reflection-based attributes on the variant.
    pub attrs: ReflectFieldAttr,
    /// The index of this variant within the enum.
    #[allow(dead_code)]
    pub index: usize,
}

pub(crate) enum EnumVariantFields<'a> {
    Named(Vec<StructField<'a>>),
    Unnamed(Vec<StructField<'a>>),
    Unit,
}

impl<'a> ReflectDerive<'a> {
    pub fn from_input(input: &'a DeriveInput) -> Result<Self, syn::Error> {
        let mut traits = ReflectTraits::default();
        // Should indicate whether `#[reflect_value]` was used
        let mut force_reflect_value = false;

        for attribute in input.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
            let meta_list = if let Meta::List(meta_list) = attribute {
                meta_list
            } else {
                continue;
            };

            if let Some(ident) = meta_list.path.get_ident() {
                if ident == REFLECT_ATTRIBUTE_NAME {
                    traits = ReflectTraits::from_nested_metas(&meta_list.nested);
                } else if ident == REFLECT_VALUE_ATTRIBUTE_NAME {
                    force_reflect_value = true;
                    traits = ReflectTraits::from_nested_metas(&meta_list.nested);
                }
            }
        }

        let meta = ReflectMeta::new(&input.ident, &input.generics, traits);

        if force_reflect_value {
            return Ok(Self::Value(meta));
        }

        return match &input.data {
            Data::Struct(data) => {
                let reflect_struct = ReflectStruct {
                    meta,
                    fields: Self::collect_struct_fields(&data.fields)?,
                };

                match data.fields {
                    Fields::Named(..) => Ok(Self::Struct(reflect_struct)),
                    Fields::Unnamed(..) => Ok(Self::TupleStruct(reflect_struct)),
                    Fields::Unit => Ok(Self::UnitStruct(reflect_struct)),
                }
            }
            Data::Enum(data) => {
                let reflect_enum = ReflectEnum {
                    meta,
                    variants: Self::collect_enum_variants(&data.variants)?,
                };
                Ok(Self::Enum(reflect_enum))
            }
            Data::Union(..) => Err(syn::Error::new(
                input.span(),
                "reflection not supported for unions",
            )),
        };
    }

    fn collect_struct_fields(fields: &'a Fields) -> Result<Vec<StructField<'a>>, syn::Error> {
        let sifter: utility::ResultSifter<StructField<'a>> = fields
            .iter()
            .enumerate()
            .map(|(index, field)| -> Result<StructField, syn::Error> {
                let attrs = parse_field_attrs(&field.attrs)?;
                Ok(StructField {
                    index,
                    attrs,
                    data: field,
                })
            })
            .fold(
                utility::ResultSifter::default(),
                utility::ResultSifter::fold,
            );

        sifter.finish()
    }

    fn collect_enum_variants(
        variants: &'a Punctuated<Variant, Token![,]>,
    ) -> Result<Vec<EnumVariant<'a>>, syn::Error> {
        let sifter: utility::ResultSifter<EnumVariant<'a>> = variants
            .iter()
            .enumerate()
            .map(|(index, variant)| -> Result<EnumVariant, syn::Error> {
                let attrs = parse_field_attrs(&variant.attrs)?;
                let fields = Self::collect_struct_fields(&variant.fields)?;

                Ok(match variant.fields {
                    Fields::Named(..) => EnumVariant {
                        data: variant,
                        fields: EnumVariantFields::Named(fields),
                        attrs,
                        index,
                    },
                    Fields::Unnamed(..) => EnumVariant {
                        data: variant,
                        fields: EnumVariantFields::Unnamed(fields),
                        attrs,
                        index,
                    },
                    Fields::Unit => EnumVariant {
                        data: variant,
                        fields: EnumVariantFields::Unit,
                        attrs,
                        index,
                    },
                })
            })
            .fold(
                utility::ResultSifter::default(),
                utility::ResultSifter::fold,
            );

        sifter.finish()
    }
}

impl<'a> ReflectMeta<'a> {
    pub fn new(type_name: &'a Ident, generics: &'a Generics, traits: ReflectTraits) -> Self {
        Self {
            traits,
            type_name,
            generics,
            bevy_reflect_path: utility::get_bevy_reflect_path(),
        }
    }

    /// The registered reflect traits on this struct.
    pub fn traits(&self) -> &ReflectTraits {
        &self.traits
    }

    /// The name of this struct.
    pub fn type_name(&self) -> &'a Ident {
        self.type_name
    }

    /// The generics associated with this struct.
    pub fn generics(&self) -> &'a Generics {
        self.generics
    }

    /// The cached `bevy_reflect` path.
    pub fn bevy_reflect_path(&self) -> &Path {
        &self.bevy_reflect_path
    }

    /// Returns the `GetTypeRegistration` impl as a `TokenStream`.
    pub fn get_type_registration(&self) -> proc_macro2::TokenStream {
        crate::registration::impl_get_type_registration(
            self.type_name,
            &self.bevy_reflect_path,
            self.traits.idents(),
            self.generics,
        )
    }
}

impl<'a> ReflectStruct<'a> {
    /// Access the metadata associated with this struct definition.
    pub fn meta(&self) -> &ReflectMeta<'a> {
        &self.meta
    }

    /// Get an iterator over the active fields.
    pub fn active_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields.iter().filter(|field| !field.attrs.ignore)
    }

    /// Get an iterator over the ignored fields.
    pub fn ignored_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields.iter().filter(|field| field.attrs.ignore)
    }

    /// Get a collection of all active types.
    pub fn active_types(&self) -> Vec<syn::Type> {
        self.active_fields()
            .map(|field| field.data.ty.clone())
            .collect::<Vec<_>>()
    }

    /// The complete set of fields in this struct.
    #[allow(dead_code)]
    pub fn fields(&self) -> &[StructField<'a>] {
        &self.fields
    }
}

impl<'a> ReflectEnum<'a> {
    /// Access the metadata associated with this enum definition.
    pub fn meta(&self) -> &ReflectMeta<'a> {
        &self.meta
    }

    /// Get an iterator over the active variants.
    pub fn active_variants(&self) -> impl Iterator<Item = &EnumVariant<'a>> {
        self.variants.iter().filter(|variant| !variant.attrs.ignore)
    }

    /// Get an iterator over the ignored variants.
    #[allow(dead_code)]
    pub fn ignored_variants(&self) -> impl Iterator<Item = &EnumVariant<'a>> {
        self.variants.iter().filter(|variant| variant.attrs.ignore)
    }

    /// Returns the given ident as a qualified unit variant of this enum.
    pub fn get_unit(&self, variant: &Ident) -> proc_macro2::TokenStream {
        let name = self.meta.type_name;
        quote! {
            #name::#variant
        }
    }

    /// The complete set of variants in this enum.
    #[allow(dead_code)]
    pub fn variants(&self) -> &[EnumVariant<'a>] {
        &self.variants
    }
}
