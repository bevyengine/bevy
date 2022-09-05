use crate::container_attributes::ReflectTraits;
use crate::field_attributes::{parse_field_attrs, ReflectFieldAttr};
use quote::quote;

use crate::{
    utility, REFLECT_ATTRIBUTE_NAME, REFLECT_VALUE_ATTRIBUTE_NAME, TYPE_PATH_ATTRIBUTE_NAME,
};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Field, Fields, Generics, Ident, Lit, Meta, MetaList, NestedMeta, Path,
    Token, Type, Variant,
};

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
    /// User defined options for the impl of `TypePath`.
    type_path_options: TypePathOptions,
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

        let mut type_path_options = None;

        for attribute in input.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
            let meta_list = if let Meta::List(meta_list) = attribute {
                meta_list
            } else {
                continue;
            };

            match meta_list.path.get_ident() {
                Some(ident) if ident == REFLECT_ATTRIBUTE_NAME => {
                    traits = ReflectTraits::from_nested_metas(&meta_list.nested);
                }
                Some(ident) if ident == REFLECT_VALUE_ATTRIBUTE_NAME => {
                    force_reflect_value = true;
                    traits = ReflectTraits::from_nested_metas(&meta_list.nested);
                }
                Some(ident) if ident == TYPE_PATH_ATTRIBUTE_NAME => {
                    type_path_options = Some(TypePathOptions::parse_meta_list(meta_list)?);
                }
                _ => {}
            }
        }

        let meta = ReflectMeta::new(
            &input.ident,
            &input.generics,
            traits,
            type_path_options.unwrap_or_default(),
        );

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
                let fields = Self::collect_struct_fields(&variant.fields)?;

                let fields = match variant.fields {
                    Fields::Named(..) => EnumVariantFields::Named(fields),
                    Fields::Unnamed(..) => EnumVariantFields::Unnamed(fields),
                    Fields::Unit => EnumVariantFields::Unit,
                };
                Ok(EnumVariant {
                    fields,
                    attrs: parse_field_attrs(&variant.attrs)?,
                    data: variant,
                    index,
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
    pub fn new(
        type_name: &'a Ident,
        generics: &'a Generics,
        traits: ReflectTraits,
        type_path_options: TypePathOptions,
    ) -> Self {
        Self {
            traits,
            type_name,
            generics,
            type_path_options,
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

    /// User defined options for the impl of `TypePath`.
    pub fn type_path_options(&self) -> &TypePathOptions {
        &self.type_path_options
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
    pub fn active_types(&self) -> impl Iterator<Item = &Type> {
        self.active_fields().map(|field| &field.data.ty)
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

/// User defined options for the impl of `TypePath`.
#[derive(Default)]
pub(crate) struct TypePathOptions {
    /// If set, the custom module path.
    pub module_path: Option<String>,
    /// If set, the custom type ident.
    pub type_ident: Option<String>,
}

impl TypePathOptions {
    fn parse_meta_list(meta_list: MetaList) -> Result<Self, syn::Error> {
        fn parse_module_path(lit: Lit) -> Result<String, syn::Error> {
            fn is_valid_module_path(_module_path: &str) -> bool {
                // FIXME: what conditions here ?
                true
            }

            match lit {
                Lit::Str(lit_str) => {
                    let path = lit_str.value();
                    if is_valid_module_path(&path) {
                        Ok(path)
                    } else {
                        Err(syn::Error::new(
                            lit_str.span(),
                            format!("Expected a valid module path"),
                        ))
                    }
                }
                other => Err(syn::Error::new(
                    other.span(),
                    format!("Expected a str literal"),
                )),
            }
        }

        fn parse_tpye_ident(lit: Lit) -> Result<String, syn::Error> {
            fn is_valid_type_ident(_type_ident: &str) -> bool {
                // FIXME: what conditions here ?
                true
            }

            match lit {
                Lit::Str(lit_str) => {
                    let type_ident = lit_str.value();
                    if is_valid_type_ident(&type_ident) {
                        Ok(type_ident)
                    } else {
                        Err(syn::Error::new(
                            lit_str.span(),
                            format!("Expected a valid type ident"),
                        ))
                    }
                }
                other => Err(syn::Error::new(
                    other.span(),
                    format!("Expected a str literal"),
                )),
            }
        }

        let mut module_path = None;
        let mut type_ident = None;

        for attribute in meta_list.nested {
            match attribute {
                NestedMeta::Meta(Meta::NameValue(name_value)) => {
                    if let Some(ident) = name_value.path.get_ident() {
                        if ident == "path" {
                            module_path = Some(parse_module_path(name_value.lit)?);
                        } else if ident == "ident" {
                            type_ident = Some(parse_tpye_ident(name_value.lit)?);
                        }
                    } else {
                        return Err(syn::Error::new(
                            name_value.path.span(),
                            format!("Unexpected entry for the `{TYPE_PATH_ATTRIBUTE_NAME}` attribute. Usage: #[{TYPE_PATH_ATTRIBUTE_NAME}(path = \"my_crate::my_module\", ident = \"MyType\")]"),
                        ));
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        other.span(),
                        format!("Unexpected entry for the `{TYPE_PATH_ATTRIBUTE_NAME}` attribute. Usage: #[{TYPE_PATH_ATTRIBUTE_NAME}(path = \"my_crate::my_module\", ident = \"MyType\")]"),
                    ));
                }
            }
        }

        Ok(Self {
            module_path,
            type_ident,
        })
    }
}
