use core::fmt;
use proc_macro2::Span;

use crate::container_attributes::{ContainerAttributes, FromReflectAttrs, TypePathAttrs};
use crate::field_attributes::FieldAttributes;
use crate::type_path::parse_path_no_leading_colon;
use crate::utility::{StringExpr, WhereClauseOptions};
use quote::{quote, ToTokens};
use syn::token::Comma;

use crate::serialization::SerializationDataDef;
use crate::{
    utility, REFLECT_ATTRIBUTE_NAME, REFLECT_VALUE_ATTRIBUTE_NAME, TYPE_NAME_ATTRIBUTE_NAME,
    TYPE_PATH_ATTRIBUTE_NAME,
};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parse_str, Data, DeriveInput, Field, Fields, GenericParam, Generics, Ident, LitStr, Meta, Path,
    PathSegment, Type, TypeParam, Variant,
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
/// ```ignore (bevy_reflect is not accessible from this crate)
/// #[derive(Reflect)]
/// //                          traits
/// //        |----------------------------------------|
/// #[reflect(PartialEq, Serialize, Deserialize, Default)]
/// //            type_path       generics
/// //     |-------------------||----------|
/// struct ThingThatImReflecting<T1, T2, T3> {/* ... */}
/// ```
pub(crate) struct ReflectMeta<'a> {
    /// The registered traits for this type.
    attrs: ContainerAttributes,
    /// The path to this type.
    type_path: ReflectTypePath<'a>,
    /// A cached instance of the path to the `bevy_reflect` crate.
    bevy_reflect_path: Path,
    /// The documentation for this type, if any
    #[cfg(feature = "documentation")]
    docs: crate::documentation::Documentation,
}

/// Struct data used by derive macros for `Reflect` and `FromReflect`.
///
/// # Example
///
/// ```ignore (bevy_reflect is not accessible from this crate)
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
    serialization_data: Option<SerializationDataDef>,
    fields: Vec<StructField<'a>>,
}

/// Enum data used by derive macros for `Reflect` and `FromReflect`.
///
/// # Example
///
/// ```ignore (bevy_reflect is not accessible from this crate)
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
#[derive(Clone)]
pub(crate) struct StructField<'a> {
    /// The raw field.
    pub data: &'a Field,
    /// The reflection-based attributes on the field.
    pub attrs: FieldAttributes,
    /// The index of this field within the struct.
    pub declaration_index: usize,
    /// The index of this field as seen by the reflection API.
    ///
    /// This index accounts for the removal of [ignored] fields.
    /// It will only be `Some(index)` when the field is not ignored.
    ///
    /// [ignored]: crate::field_attributes::ReflectIgnoreBehavior::IgnoreAlways
    pub reflection_index: Option<usize>,
    /// The documentation for this field, if any
    #[cfg(feature = "documentation")]
    pub doc: crate::documentation::Documentation,
}

/// Represents a variant on an enum.
pub(crate) struct EnumVariant<'a> {
    /// The raw variant.
    pub data: &'a Variant,
    /// The fields within this variant.
    pub fields: EnumVariantFields<'a>,
    /// The reflection-based attributes on the variant.
    #[allow(dead_code)]
    pub attrs: FieldAttributes,
    /// The index of this variant within the enum.
    #[allow(dead_code)]
    pub index: usize,
    /// The documentation for this variant, if any
    #[cfg(feature = "documentation")]
    pub doc: crate::documentation::Documentation,
}

pub(crate) enum EnumVariantFields<'a> {
    Named(Vec<StructField<'a>>),
    Unnamed(Vec<StructField<'a>>),
    Unit,
}

/// The method in which the type should be reflected.
#[derive(PartialEq, Eq)]
enum ReflectMode {
    /// Reflect the type normally, providing information about all fields/variants.
    Normal,
    /// Reflect the type as a value.
    Value,
}

/// How the macro was invoked.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ReflectImplSource {
    ImplRemoteType,
    DeriveLocalType,
}

/// Which trait the macro explicitly implements.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ReflectTraitToImpl {
    Reflect,
    FromReflect,
    TypePath,
}

/// The provenance of a macro invocation.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct ReflectProvenance {
    pub source: ReflectImplSource,
    pub trait_: ReflectTraitToImpl,
}

impl fmt::Display for ReflectProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::{ReflectImplSource as S, ReflectTraitToImpl as T};
        let str = match (self.source, self.trait_) {
            (S::ImplRemoteType, T::Reflect) => "`impl_reflect`",
            (S::DeriveLocalType, T::Reflect) => "`#[derive(Reflect)]`",
            (S::DeriveLocalType, T::FromReflect) => "`#[derive(FromReflect)]`",
            (S::DeriveLocalType, T::TypePath) => "`#[derive(TypePath)]`",
            (S::ImplRemoteType, T::FromReflect | T::TypePath) => unreachable!(),
        };
        f.write_str(str)
    }
}

impl<'a> ReflectDerive<'a> {
    pub fn from_input(
        input: &'a DeriveInput,
        provenance: ReflectProvenance,
    ) -> Result<Self, syn::Error> {
        let mut container_attributes = ContainerAttributes::default();
        // Should indicate whether `#[reflect_value]` was used.
        let mut reflect_mode = None;
        // Should indicate whether `#[type_path = "..."]` was used.
        let mut custom_path: Option<Path> = None;
        // Should indicate whether `#[type_name = "..."]` was used.
        let mut custom_type_name: Option<Ident> = None;

        #[cfg(feature = "documentation")]
        let mut doc = crate::documentation::Documentation::default();

        for attribute in &input.attrs {
            match &attribute.meta {
                Meta::List(meta_list) if meta_list.path.is_ident(REFLECT_ATTRIBUTE_NAME) => {
                    if !matches!(reflect_mode, None | Some(ReflectMode::Normal)) {
                        return Err(syn::Error::new(
                            meta_list.span(),
                            format_args!("cannot use both `#[{REFLECT_ATTRIBUTE_NAME}]` and `#[{REFLECT_VALUE_ATTRIBUTE_NAME}]`"),
                        ));
                    }

                    reflect_mode = Some(ReflectMode::Normal);
                    container_attributes.parse_meta_list(meta_list, provenance.trait_)?;
                }
                Meta::List(meta_list) if meta_list.path.is_ident(REFLECT_VALUE_ATTRIBUTE_NAME) => {
                    if !matches!(reflect_mode, None | Some(ReflectMode::Value)) {
                        return Err(syn::Error::new(
                            meta_list.span(),
                            format_args!("cannot use both `#[{REFLECT_ATTRIBUTE_NAME}]` and `#[{REFLECT_VALUE_ATTRIBUTE_NAME}]`"),
                        ));
                    }

                    reflect_mode = Some(ReflectMode::Value);
                    container_attributes.parse_meta_list(meta_list, provenance.trait_)?;
                }
                Meta::Path(path) if path.is_ident(REFLECT_VALUE_ATTRIBUTE_NAME) => {
                    if !matches!(reflect_mode, None | Some(ReflectMode::Value)) {
                        return Err(syn::Error::new(
                            path.span(),
                            format_args!("cannot use both `#[{REFLECT_ATTRIBUTE_NAME}]` and `#[{REFLECT_VALUE_ATTRIBUTE_NAME}]`"),
                        ));
                    }

                    reflect_mode = Some(ReflectMode::Value);
                }
                Meta::NameValue(pair) if pair.path.is_ident(TYPE_PATH_ATTRIBUTE_NAME) => {
                    let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit),
                        ..
                    }) = &pair.value
                    else {
                        return Err(syn::Error::new(
                            pair.span(),
                            format_args!("`#[{TYPE_PATH_ATTRIBUTE_NAME} = \"...\"]` must be a string literal"),
                        ));
                    };

                    custom_path = Some(syn::parse::Parser::parse_str(
                        parse_path_no_leading_colon,
                        &lit.value(),
                    )?);
                }
                Meta::NameValue(pair) if pair.path.is_ident(TYPE_NAME_ATTRIBUTE_NAME) => {
                    let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit),
                        ..
                    }) = &pair.value
                    else {
                        return Err(syn::Error::new(
                            pair.span(),
                            format_args!("`#[{TYPE_NAME_ATTRIBUTE_NAME} = \"...\"]` must be a string literal"),
                        ));
                    };

                    custom_type_name = Some(parse_str(&lit.value())?);
                }
                #[cfg(feature = "documentation")]
                Meta::NameValue(pair) if pair.path.is_ident("doc") => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit),
                        ..
                    }) = &pair.value
                    {
                        doc.push(lit.value());
                    }
                }
                _ => continue,
            }
        }
        match (&mut custom_path, custom_type_name) {
            (Some(path), custom_type_name) => {
                let ident = custom_type_name.unwrap_or_else(|| input.ident.clone());
                path.segments.push(PathSegment::from(ident));
            }
            (None, Some(name)) => {
                return Err(syn::Error::new(
                    name.span(),
                    format!("cannot use `#[{TYPE_NAME_ATTRIBUTE_NAME} = \"...\"]` without a `#[{TYPE_PATH_ATTRIBUTE_NAME} = \"...\"]` attribute."),
                ));
            }
            _ => (),
        }

        let type_path = ReflectTypePath::Internal {
            ident: &input.ident,
            custom_path,
            generics: &input.generics,
        };

        let meta = ReflectMeta::new(type_path, container_attributes);

        if provenance.source == ReflectImplSource::ImplRemoteType
            && meta.type_path_attrs().should_auto_derive()
            && !meta.type_path().has_custom_path()
        {
            return Err(syn::Error::new(
                meta.type_path().span(),
                format!("a #[{TYPE_PATH_ATTRIBUTE_NAME} = \"...\"] attribute must be specified when using {provenance}")
            ));
        }

        #[cfg(feature = "documentation")]
        let meta = meta.with_docs(doc);

        // Use normal reflection if unspecified
        let reflect_mode = reflect_mode.unwrap_or(ReflectMode::Normal);

        if reflect_mode == ReflectMode::Value {
            return Ok(Self::Value(meta));
        }

        return match &input.data {
            Data::Struct(data) => {
                let fields = Self::collect_struct_fields(&data.fields)?;
                let reflect_struct = ReflectStruct {
                    meta,
                    serialization_data: SerializationDataDef::new(&fields)?,
                    fields,
                };

                match data.fields {
                    Fields::Named(..) => Ok(Self::Struct(reflect_struct)),
                    Fields::Unnamed(..) => Ok(Self::TupleStruct(reflect_struct)),
                    Fields::Unit => Ok(Self::UnitStruct(reflect_struct)),
                }
            }
            Data::Enum(data) => {
                let variants = Self::collect_enum_variants(&data.variants)?;

                let reflect_enum = ReflectEnum { meta, variants };
                Ok(Self::Enum(reflect_enum))
            }
            Data::Union(..) => Err(syn::Error::new(
                input.span(),
                "reflection not supported for unions",
            )),
        };
    }

    pub fn meta(&self) -> &ReflectMeta<'a> {
        match self {
            ReflectDerive::Struct(data)
            | ReflectDerive::TupleStruct(data)
            | ReflectDerive::UnitStruct(data) => data.meta(),
            ReflectDerive::Enum(data) => data.meta(),
            ReflectDerive::Value(meta) => meta,
        }
    }

    fn collect_struct_fields(fields: &'a Fields) -> Result<Vec<StructField<'a>>, syn::Error> {
        let mut active_index = 0;
        let sifter: utility::ResultSifter<StructField<'a>> = fields
            .iter()
            .enumerate()
            .map(
                |(declaration_index, field)| -> Result<StructField, syn::Error> {
                    let attrs = FieldAttributes::parse_attributes(&field.attrs)?;

                    let reflection_index = if attrs.ignore.is_ignored() {
                        None
                    } else {
                        active_index += 1;
                        Some(active_index - 1)
                    };

                    Ok(StructField {
                        declaration_index,
                        reflection_index,
                        attrs,
                        data: field,
                        #[cfg(feature = "documentation")]
                        doc: crate::documentation::Documentation::from_attributes(&field.attrs),
                    })
                },
            )
            .fold(
                utility::ResultSifter::default(),
                utility::ResultSifter::fold,
            );

        sifter.finish()
    }

    fn collect_enum_variants(
        variants: &'a Punctuated<Variant, Comma>,
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
                    attrs: FieldAttributes::parse_attributes(&variant.attrs)?,
                    data: variant,
                    index,
                    #[cfg(feature = "documentation")]
                    doc: crate::documentation::Documentation::from_attributes(&variant.attrs),
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
    pub fn new(type_path: ReflectTypePath<'a>, attrs: ContainerAttributes) -> Self {
        Self {
            attrs,
            type_path,
            bevy_reflect_path: utility::get_bevy_reflect_path(),
            #[cfg(feature = "documentation")]
            docs: Default::default(),
        }
    }

    /// Sets the documentation for this type.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: crate::documentation::Documentation) -> Self {
        Self { docs, ..self }
    }

    /// The registered reflect attributes on this struct.
    pub fn attrs(&self) -> &ContainerAttributes {
        &self.attrs
    }

    /// The `FromReflect` attributes on this type.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_reflect(&self) -> &FromReflectAttrs {
        self.attrs.from_reflect_attrs()
    }

    /// The `TypePath` attributes on this type.
    pub fn type_path_attrs(&self) -> &TypePathAttrs {
        self.attrs.type_path_attrs()
    }

    /// The path to this type.
    pub fn type_path(&self) -> &ReflectTypePath<'a> {
        &self.type_path
    }

    /// The cached `bevy_reflect` path.
    pub fn bevy_reflect_path(&self) -> &Path {
        &self.bevy_reflect_path
    }

    /// Returns the `GetTypeRegistration` impl as a `TokenStream`.
    pub fn get_type_registration(
        &self,
        where_clause_options: &WhereClauseOptions,
    ) -> proc_macro2::TokenStream {
        crate::registration::impl_get_type_registration(
            self,
            where_clause_options,
            None,
            Option::<std::iter::Empty<&Type>>::None,
        )
    }

    /// The collection of docstrings for this type, if any.
    #[cfg(feature = "documentation")]
    pub fn doc(&self) -> &crate::documentation::Documentation {
        &self.docs
    }
}

impl<'a> StructField<'a> {
    /// Generates a `TokenStream` for `NamedField` or `UnnamedField` construction.
    pub fn to_info_tokens(&self, bevy_reflect_path: &Path) -> proc_macro2::TokenStream {
        let name = match &self.data.ident {
            Some(ident) => ident.to_string().to_token_stream(),
            None => self.reflection_index.to_token_stream(),
        };

        let field_info = if self.data.ident.is_some() {
            quote! {
                #bevy_reflect_path::NamedField
            }
        } else {
            quote! {
                #bevy_reflect_path::UnnamedField
            }
        };

        let ty = &self.data.ty;
        let custom_attributes = self.attrs.custom_attributes.to_tokens(bevy_reflect_path);

        #[allow(unused_mut)] // Needs mutability for the feature gate
        let mut info = quote! {
            #field_info::new::<#ty>(#name).with_custom_attributes(#custom_attributes)
        };

        #[cfg(feature = "documentation")]
        {
            let docs = &self.doc;
            info.extend(quote! {
                .with_docs(#docs)
            });
        }

        info
    }
}

impl<'a> ReflectStruct<'a> {
    /// Access the metadata associated with this struct definition.
    pub fn meta(&self) -> &ReflectMeta<'a> {
        &self.meta
    }

    /// Returns the [`SerializationDataDef`] for this struct.
    pub fn serialization_data(&self) -> Option<&SerializationDataDef> {
        self.serialization_data.as_ref()
    }

    /// Returns the `GetTypeRegistration` impl as a `TokenStream`.
    ///
    /// Returns a specific implementation for structs and this method should be preferred over the generic [`get_type_registration`](ReflectMeta) method
    pub fn get_type_registration(
        &self,
        where_clause_options: &WhereClauseOptions,
    ) -> proc_macro2::TokenStream {
        crate::registration::impl_get_type_registration(
            self.meta(),
            where_clause_options,
            self.serialization_data(),
            Some(self.active_types().iter()),
        )
    }

    /// Get a collection of types which are exposed to the reflection API
    pub fn active_types(&self) -> Vec<Type> {
        self.active_fields()
            .map(|field| field.data.ty.clone())
            .collect()
    }

    /// Get an iterator of fields which are exposed to the reflection API.
    pub fn active_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields()
            .iter()
            .filter(|field| field.attrs.ignore.is_active())
    }

    /// Get an iterator of fields which are ignored by the reflection API
    pub fn ignored_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields()
            .iter()
            .filter(|field| field.attrs.ignore.is_ignored())
    }

    /// The complete set of fields in this struct.
    pub fn fields(&self) -> &[StructField<'a>] {
        &self.fields
    }

    pub fn where_clause_options(&self) -> WhereClauseOptions {
        WhereClauseOptions::new_with_fields(self.meta(), self.active_types().into_boxed_slice())
    }

    /// Generates a `TokenStream` for `TypeInfo::Struct` or `TypeInfo::TupleStruct` construction.
    pub fn to_info_tokens(&self, is_tuple: bool) -> proc_macro2::TokenStream {
        let bevy_reflect_path = self.meta().bevy_reflect_path();

        let (info_variant, info_struct) = if is_tuple {
            (
                Ident::new("TupleStruct", Span::call_site()),
                Ident::new("TupleStructInfo", Span::call_site()),
            )
        } else {
            (
                Ident::new("Struct", Span::call_site()),
                Ident::new("StructInfo", Span::call_site()),
            )
        };

        let field_infos = self
            .active_fields()
            .map(|field| field.to_info_tokens(bevy_reflect_path));

        let custom_attributes = self
            .meta
            .attrs
            .custom_attributes()
            .to_tokens(bevy_reflect_path);

        #[allow(unused_mut)] // Needs mutability for the feature gate
        let mut info = quote! {
            #bevy_reflect_path::#info_struct::new::<Self>(&[
                #(#field_infos),*
            ])
            .with_custom_attributes(#custom_attributes)
        };

        #[cfg(feature = "documentation")]
        {
            let docs = self.meta().doc();
            info.extend(quote! {
                .with_docs(#docs)
            });
        }

        quote! {
            #bevy_reflect_path::TypeInfo::#info_variant(#info)
        }
    }
}

impl<'a> ReflectEnum<'a> {
    /// Access the metadata associated with this enum definition.
    pub fn meta(&self) -> &ReflectMeta<'a> {
        &self.meta
    }

    /// Returns the given ident as a qualified unit variant of this enum.
    pub fn get_unit(&self, variant: &Ident) -> proc_macro2::TokenStream {
        let name = self.meta.type_path();
        quote! {
            #name::#variant
        }
    }

    /// The complete set of variants in this enum.
    pub fn variants(&self) -> &[EnumVariant<'a>] {
        &self.variants
    }

    /// Get a collection of types which are exposed to the reflection API
    pub fn active_types(&self) -> Vec<Type> {
        self.active_fields()
            .map(|field| field.data.ty.clone())
            .collect()
    }

    /// Get an iterator of fields which are exposed to the reflection API
    pub fn active_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.variants
            .iter()
            .flat_map(|variant| variant.active_fields())
    }

    pub fn where_clause_options(&self) -> WhereClauseOptions {
        WhereClauseOptions::new_with_fields(self.meta(), self.active_types().into_boxed_slice())
    }

    /// Returns the `GetTypeRegistration` impl as a `TokenStream`.
    ///
    /// Returns a specific implementation for enums and this method should be preferred over the generic [`get_type_registration`](crate::ReflectMeta) method
    pub fn get_type_registration(
        &self,
        where_clause_options: &WhereClauseOptions,
    ) -> proc_macro2::TokenStream {
        crate::registration::impl_get_type_registration(
            self.meta(),
            where_clause_options,
            None,
            Some(self.active_fields().map(|field| &field.data.ty)),
        )
    }

    /// Generates a `TokenStream` for `TypeInfo::Enum` construction.
    pub fn to_info_tokens(&self) -> proc_macro2::TokenStream {
        let bevy_reflect_path = self.meta().bevy_reflect_path();

        let variants = self
            .variants
            .iter()
            .map(|variant| variant.to_info_tokens(bevy_reflect_path));

        let custom_attributes = self
            .meta
            .attrs
            .custom_attributes()
            .to_tokens(bevy_reflect_path);

        #[allow(unused_mut)] // Needs mutability for the feature gate
        let mut info = quote! {
            #bevy_reflect_path::EnumInfo::new::<Self>(&[
                #(#variants),*
            ])
            .with_custom_attributes(#custom_attributes)
        };

        #[cfg(feature = "documentation")]
        {
            let docs = self.meta().doc();
            info.extend(quote! {
                .with_docs(#docs)
            });
        }

        quote! {
            #bevy_reflect_path::TypeInfo::Enum(#info)
        }
    }
}

impl<'a> EnumVariant<'a> {
    /// Get an iterator of fields which are exposed to the reflection API
    pub fn active_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields()
            .iter()
            .filter(|field| field.attrs.ignore.is_active())
    }

    /// The complete set of fields in this variant.
    #[allow(dead_code)]
    pub fn fields(&self) -> &[StructField<'a>] {
        match &self.fields {
            EnumVariantFields::Named(fields) | EnumVariantFields::Unnamed(fields) => fields,
            EnumVariantFields::Unit => &[],
        }
    }

    /// Generates a `TokenStream` for `VariantInfo` construction.
    pub fn to_info_tokens(&self, bevy_reflect_path: &Path) -> proc_macro2::TokenStream {
        let variant_name = &self.data.ident.to_string();

        let (info_variant, info_struct) = match &self.fields {
            EnumVariantFields::Unit => (
                Ident::new("Unit", Span::call_site()),
                Ident::new("UnitVariantInfo", Span::call_site()),
            ),
            EnumVariantFields::Unnamed(..) => (
                Ident::new("Tuple", Span::call_site()),
                Ident::new("TupleVariantInfo", Span::call_site()),
            ),
            EnumVariantFields::Named(..) => (
                Ident::new("Struct", Span::call_site()),
                Ident::new("StructVariantInfo", Span::call_site()),
            ),
        };

        let fields = self
            .active_fields()
            .map(|field| field.to_info_tokens(bevy_reflect_path));

        let args = match &self.fields {
            EnumVariantFields::Unit => quote!(#variant_name),
            _ => {
                quote!( #variant_name , &[#(#fields),*] )
            }
        };

        let custom_attributes = self.attrs.custom_attributes.to_tokens(bevy_reflect_path);

        #[allow(unused_mut)] // Needs mutability for the feature gate
        let mut info = quote! {
            #bevy_reflect_path::#info_struct::new(#args)
                .with_custom_attributes(#custom_attributes)
        };

        #[cfg(feature = "documentation")]
        {
            let docs = &self.doc;
            info.extend(quote! {
                .with_docs(#docs)
            });
        }

        quote! {
            #bevy_reflect_path::VariantInfo::#info_variant(#info)
        }
    }
}

/// Represents a path to a type.
///
/// This is used over [`struct@Ident`] or [`Path`]
/// to have the correct semantics for [deriving `TypePath`].
///
/// The type can always be reached with its [`ToTokens`] implementation.
///
/// The [`short_type_path`], [`type_ident`], [`crate_name`], and [`module_path`] methods
/// have corresponding methods on the `TypePath` trait.
/// [`long_type_path`] corresponds to the `type_path` method on `TypePath`.
///
/// [deriving `TypePath`]: crate::derive_type_path
/// [`long_type_path`]: ReflectTypePath::long_type_path
/// [`short_type_path`]: ReflectTypePath::short_type_path
/// [`type_ident`]: ReflectTypePath::type_ident
/// [`crate_name`]: ReflectTypePath::crate_name
/// [`module_path`]: ReflectTypePath::module_path
///
/// # Example
///
/// ```ignore  (bevy_reflect is not accessible from this crate)
/// # use syn::parse_quote;
/// # use bevy_reflect_derive::ReflectTypePath;
/// let path: syn::Path = parse_quote!(::core::marker::PhantomData)?;
///
/// let type_path = ReflectTypePath::External {
///     path,
///     custom_path: None,
/// };
///
/// // Equivalent to "core::marker".
/// let module_path = type_path.module_path();
/// # Ok::<(), syn::Error>(())
/// ```
///
pub(crate) enum ReflectTypePath<'a> {
    /// Types without a crate/module that can be named from any scope (e.g. `bool`).
    Primitive(&'a Ident),
    /// Using `::my_crate::foo::Bar` syntax.
    ///
    /// May have a separate custom path used for the `TypePath` implementation.
    External {
        path: &'a Path,
        custom_path: Option<Path>,
        generics: &'a Generics,
    },
    /// The name of a type relative to its scope.
    ///
    /// The type must be able to be reached with just its name.
    ///
    /// May have a separate alias path used for the `TypePath` implementation.
    ///
    /// Module and crate are found with [`module_path!()`](module_path),
    /// if there is no custom path specified.
    Internal {
        ident: &'a Ident,
        custom_path: Option<Path>,
        generics: &'a Generics,
    },
    /// Any [`Type`] with only a defined `type_path` and `short_type_path`.
    #[allow(dead_code)]
    // Not currently used but may be useful in the future due to its generality.
    Anonymous {
        qualified_type: Type,
        long_type_path: StringExpr,
        short_type_path: StringExpr,
    },
}

impl<'a> ReflectTypePath<'a> {
    /// Returns the path interpreted as an [`struct@Ident`].
    ///
    /// Returns [`None`] if [anonymous].
    ///
    /// [anonymous]: ReflectTypePath::Anonymous
    pub fn get_ident(&self) -> Option<&Ident> {
        match self {
            Self::Internal {
                ident, custom_path, ..
            } => Some(
                custom_path
                    .as_ref()
                    .map(|path| &path.segments.last().unwrap().ident)
                    .unwrap_or(ident),
            ),
            Self::External {
                path, custom_path, ..
            } => Some(
                &custom_path
                    .as_ref()
                    .unwrap_or(path)
                    .segments
                    .last()
                    .unwrap()
                    .ident,
            ),
            Self::Primitive(ident) => Some(ident),
            _ => None,
        }
    }

    /// The generics associated with the type.
    ///
    /// Empty if [anonymous] or [primitive].
    ///
    /// [primitive]: ReflectTypePath::Primitive
    /// [anonymous]: ReflectTypePath::Anonymous
    pub fn generics(&self) -> &'a Generics {
        // Use a constant because we need to return a reference of at least 'a.
        const EMPTY_GENERICS: &Generics = &Generics {
            gt_token: None,
            lt_token: None,
            where_clause: None,
            params: Punctuated::new(),
        };

        match self {
            Self::Internal { generics, .. } | Self::External { generics, .. } => generics,
            _ => EMPTY_GENERICS,
        }
    }

    /// Whether an implementation of `Typed` or `TypePath` should be generic.
    ///
    /// Returning true that it should use a `GenericTypeCell` in its implementation.
    pub fn impl_is_generic(&self) -> bool {
        // Whether to use `GenericTypeCell` is not dependent on lifetimes
        // (which all have to be 'static anyway).
        !self
            .generics()
            .params
            .iter()
            .all(|param| matches!(param, GenericParam::Lifetime(_)))
    }

    /// Returns the path interpreted as a [`Path`].
    ///
    /// Returns [`None`] if [anonymous], [primitive],
    /// or a [`ReflectTypePath::Internal`] without a custom path.
    ///
    /// [primitive]: ReflectTypePath::Primitive
    /// [anonymous]: ReflectTypePath::Anonymous
    pub fn get_path(&self) -> Option<&Path> {
        match self {
            Self::Internal { custom_path, .. } => custom_path.as_ref(),
            Self::External {
                path, custom_path, ..
            } => Some(custom_path.as_ref().unwrap_or(path)),
            _ => None,
        }
    }

    /// Returns whether this [internal] or [external] path has a custom path.
    ///
    /// [internal]: ReflectTypePath::Internal
    /// [external]: ReflectTypePath::External
    pub fn has_custom_path(&self) -> bool {
        match self {
            Self::Internal { custom_path, .. } | Self::External { custom_path, .. } => {
                custom_path.is_some()
            }
            _ => false,
        }
    }

    /// Returns a [`StringExpr`] representing the name of the type's crate.
    ///
    /// Returns [`None`] if the type is [primitive] or [anonymous].
    ///
    /// For non-customised [internal] paths this is created from [`module_path`].
    ///
    /// For `Option<PhantomData>`, this is `"core"`.
    ///
    /// [primitive]: ReflectTypePath::Primitive
    /// [anonymous]: ReflectTypePath::Anonymous
    /// [internal]: ReflectTypePath::Internal
    pub fn crate_name(&self) -> Option<StringExpr> {
        if let Some(path) = self.get_path() {
            let crate_name = &path.segments.first().unwrap().ident;
            return Some(StringExpr::from(crate_name));
        }

        match self {
            Self::Internal { .. } => Some(StringExpr::Borrowed(quote! {
                ::core::module_path!()
                    .split(':')
                    .next()
                    .unwrap()
            })),
            Self::External { .. } => unreachable!(),
            _ => None,
        }
    }

    /// Combines type generics and const generics into one [`StringExpr`].
    ///
    /// This string can be used with a `GenericTypePathCell` in a `TypePath` implementation.
    ///
    /// The `ty_generic_fn` param maps [`TypeParam`]s to [`StringExpr`]s.
    fn reduce_generics(
        generics: &Generics,
        mut ty_generic_fn: impl FnMut(&TypeParam) -> StringExpr,
    ) -> StringExpr {
        let mut params = generics.params.iter().filter_map(|param| match param {
            GenericParam::Type(type_param) => Some(ty_generic_fn(type_param)),
            GenericParam::Const(const_param) => {
                let ident = &const_param.ident;
                let ty = &const_param.ty;

                Some(StringExpr::Owned(quote! {
                    <#ty as ::std::string::ToString>::to_string(&#ident)
                }))
            }
            GenericParam::Lifetime(_) => None,
        });

        params
            .next()
            .into_iter()
            .chain(params.flat_map(|x| [StringExpr::from_str(", "), x]))
            .collect()
    }

    /// Returns a [`StringExpr`] representing the "type path" of the type.
    ///
    /// For `Option<PhantomData>`, this is `"core::option::Option<core::marker::PhantomData>"`.
    pub fn long_type_path(&self, bevy_reflect_path: &Path) -> StringExpr {
        match self {
            Self::Primitive(ident) => StringExpr::from(ident),
            Self::Anonymous { long_type_path, .. } => long_type_path.clone(),
            Self::Internal { generics, .. } | Self::External { generics, .. } => {
                let ident = self.type_ident().unwrap();
                let module_path = self.module_path().unwrap();

                if self.impl_is_generic() {
                    let generics = ReflectTypePath::reduce_generics(
                        generics,
                        |TypeParam { ident, .. }| {
                            StringExpr::Borrowed(quote! {
                                <#ident as #bevy_reflect_path::TypePath>::type_path()
                            })
                        },
                    );

                    StringExpr::from_iter([
                        module_path,
                        StringExpr::from_str("::"),
                        ident,
                        StringExpr::from_str("<"),
                        generics,
                        StringExpr::from_str(">"),
                    ])
                } else {
                    StringExpr::from_iter([module_path, StringExpr::from_str("::"), ident])
                }
            }
        }
    }

    /// Returns a [`StringExpr`] representing the "short path" of the type.
    ///
    /// For `Option<PhantomData>`, this is `"Option<PhantomData>"`.
    pub fn short_type_path(&self, bevy_reflect_path: &Path) -> StringExpr {
        match self {
            Self::Anonymous {
                short_type_path, ..
            } => short_type_path.clone(),
            Self::Primitive(ident) => StringExpr::from(ident),
            Self::External { generics, .. } | Self::Internal { generics, .. } => {
                let ident = self.type_ident().unwrap();

                if self.impl_is_generic() {
                    let generics = ReflectTypePath::reduce_generics(
                        generics,
                        |TypeParam { ident, .. }| {
                            StringExpr::Borrowed(quote! {
                                <#ident as #bevy_reflect_path::TypePath>::short_type_path()
                            })
                        },
                    );

                    StringExpr::from_iter([
                        ident,
                        StringExpr::from_str("<"),
                        generics,
                        StringExpr::from_str(">"),
                    ])
                } else {
                    ident
                }
            }
        }
    }

    /// Returns a [`StringExpr`] representing the path to the module
    /// that the type is in.
    ///
    /// Returns [`None`] if the type is [primitive] or [anonymous].
    ///
    /// For non-customised [internal] paths this is created from [`module_path`].
    ///
    /// For `Option<PhantomData>`, this is `"core::option"`.
    ///
    /// [primitive]: ReflectTypePath::Primitive
    /// [anonymous]: ReflectTypePath::Anonymous
    /// [internal]: ReflectTypePath::Internal
    pub fn module_path(&self) -> Option<StringExpr> {
        if let Some(path) = self.get_path() {
            let path_string = path
                .segments
                .pairs()
                .take(path.segments.len() - 1)
                .map(|pair| pair.value().ident.to_string())
                .reduce(|path, ident| path + "::" + &ident)
                .unwrap();

            let path_lit = LitStr::new(&path_string, path.span());
            return Some(StringExpr::from_lit(&path_lit));
        }

        match self {
            Self::Internal { .. } => Some(StringExpr::Const(quote! {
                ::core::module_path!()
            })),
            Self::External { .. } => unreachable!(),
            _ => None,
        }
    }

    /// Returns a [`StringExpr`] representing the type's final ident.
    ///
    /// Returns [`None`] if the type is [anonymous].
    ///
    /// This is not necessarily a valid qualified path to the type.
    ///
    /// For `Option<PhantomData>`, this is `"Option"`.
    ///
    /// [anonymous]: ReflectTypePath::Anonymous
    pub fn type_ident(&self) -> Option<StringExpr> {
        self.get_ident().map(StringExpr::from)
    }
}

impl<'a> ToTokens for ReflectTypePath<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Internal { ident, .. } | Self::Primitive(ident) => ident.to_tokens(tokens),
            Self::External { path, .. } => path.to_tokens(tokens),
            Self::Anonymous { qualified_type, .. } => qualified_type.to_tokens(tokens),
        }
    }
}
