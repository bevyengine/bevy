use crate::container_attributes::ReflectTraits;
use crate::field_attributes::{parse_field_attrs, ReflectFieldAttr};
use crate::fq_std::{FQAny, FQDefault, FQSend, FQSync};
use crate::type_path::parse_path_no_leading_colon;
use crate::utility::{members_to_serialization_denylist, WhereClauseOptions};
use bit_set::BitSet;
use quote::{quote, ToTokens};

use crate::{
    utility, REFLECT_ATTRIBUTE_NAME, REFLECT_VALUE_ATTRIBUTE_NAME, TYPE_NAME_ATTRIBUTE_NAME,
    TYPE_PATH_ATTRIBUTE_NAME,
};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parse_str, Data, DeriveInput, Field, Fields, GenericParam, Generics, Ident, Lit, LitStr, Meta,
    Path, PathSegment, Token, Type, Variant,
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
    path_to_type: ReflectTypePath<'a>,
    /// The generics defined on this type.
    generics: &'a Generics,
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
    serialization_denylist: BitSet<u32>,
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
    pub attrs: ReflectFieldAttr,
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

impl<'a> ReflectDerive<'a> {
    pub fn from_input(input: &'a DeriveInput) -> Result<Self, syn::Error> {
        let mut traits = ReflectTraits::default();
        // Should indicate whether `#[reflect_value]` was used.
        let mut reflect_mode = None;
        // Should indicate whether `#[type_path = "..."]` was used.
        let mut custom_path: Option<Path> = None;
        // Should indicate whether `#[type_name = "..."]` was used.
        let mut custom_type_name: Option<Ident> = None;

        #[cfg(feature = "documentation")]
        let mut doc = crate::documentation::Documentation::default();

        for attribute in input.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
            match attribute {
                Meta::List(meta_list) if meta_list.path.is_ident(REFLECT_ATTRIBUTE_NAME) => {
                    if !matches!(reflect_mode, None | Some(ReflectMode::Normal)) {
                        return Err(syn::Error::new(
                            meta_list.span(),
                            format_args!("cannot use both `#[{REFLECT_ATTRIBUTE_NAME}]` and `#[{REFLECT_VALUE_ATTRIBUTE_NAME}]`"),
                        ));
                    }

                    reflect_mode = Some(ReflectMode::Normal);
                    let new_traits = ReflectTraits::from_nested_metas(&meta_list.nested)?;
                    traits = traits.merge(new_traits)?;
                }
                Meta::List(meta_list) if meta_list.path.is_ident(REFLECT_VALUE_ATTRIBUTE_NAME) => {
                    if !matches!(reflect_mode, None | Some(ReflectMode::Value)) {
                        return Err(syn::Error::new(
                            meta_list.span(),
                            format_args!("cannot use both `#[{REFLECT_ATTRIBUTE_NAME}]` and `#[{REFLECT_VALUE_ATTRIBUTE_NAME}]`"),
                        ));
                    }

                    reflect_mode = Some(ReflectMode::Value);
                    let new_traits = ReflectTraits::from_nested_metas(&meta_list.nested)?;
                    traits = traits.merge(new_traits)?;
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
                    let Lit::Str(lit) = pair.lit else {
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
                    let Lit::Str(lit) = pair.lit else {
                        return Err(syn::Error::new(
                            pair.span(),
                            format_args!("`#[{TYPE_NAME_ATTRIBUTE_NAME} = \"...\"]` must be a string literal"),
                        ));
                    };

                    custom_type_name = Some(parse_str(&lit.value())?);
                }
                #[cfg(feature = "documentation")]
                Meta::NameValue(pair) if pair.path.is_ident("doc") => {
                    if let Lit::Str(lit) = pair.lit {
                        doc.push(lit.value());
                    }
                }
                _ => continue,
            }
        }
        if let Some(path) = &mut custom_path {
            let ident = custom_type_name.unwrap_or_else(|| input.ident.clone());
            path.segments.push(PathSegment::from(ident));
        } else if let Some(name) = custom_type_name {
            return Err(syn::Error::new(
                name.span(),
                format!("cannot use `#[{TYPE_NAME_ATTRIBUTE_NAME} = \"...\"]` without a `#[{TYPE_PATH_ATTRIBUTE_NAME} = \"...\"]` attribute."),
            ));
        }

        let path_to_type = ReflectTypePath::Internal {
            ident: &input.ident,
            custom_path,
        };

        let meta = ReflectMeta::new(path_to_type, &input.generics, traits);

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
                    serialization_denylist: members_to_serialization_denylist(
                        fields.iter().map(|v| v.attrs.ignore),
                    ),
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
        let sifter: utility::ResultSifter<StructField<'a>> = fields
            .iter()
            .enumerate()
            .map(|(index, field)| -> Result<StructField, syn::Error> {
                let attrs = parse_field_attrs(&field.attrs)?;
                Ok(StructField {
                    index,
                    attrs,
                    data: field,
                    #[cfg(feature = "documentation")]
                    doc: crate::documentation::Documentation::from_attributes(&field.attrs),
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
    pub fn new(
        path_to_type: ReflectTypePath<'a>,
        generics: &'a Generics,
        traits: ReflectTraits,
    ) -> Self {
        Self {
            traits,
            path_to_type,
            generics,
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

    /// The registered reflect traits on this struct.
    pub fn traits(&self) -> &ReflectTraits {
        &self.traits
    }

    /// The name of this struct.
    pub fn path_to_type(&self) -> &'a ReflectTypePath {
        &self.path_to_type
    }

    /// The generics associated with this struct.
    pub fn generics(&self) -> &'a Generics {
        self.generics
    }

    /// The cached `bevy_reflect` path.
    pub fn bevy_reflect_path(&self) -> &Path {
        &self.bevy_reflect_path
    }

    /// Whether an impl using this [`ReflectMeta`] should be generic.
    pub fn impl_is_generic(&self) -> bool {
        // Whether to use `GenericTypedCell` is not dependent on lifetimes
        // (which all have to be 'static anyway).
        !self
            .generics
            .params
            .iter()
            .all(|param| matches!(param, GenericParam::Lifetime(_)))
    }

    /// Returns the `GetTypeRegistration` impl as a `TokenStream`.
    pub fn get_type_registration(
        &self,
        where_clause_options: &WhereClauseOptions,
    ) -> proc_macro2::TokenStream {
        crate::registration::impl_get_type_registration(self, where_clause_options, None)
    }

    /// The collection of docstrings for this type, if any.
    #[cfg(feature = "documentation")]
    pub fn doc(&self) -> &crate::documentation::Documentation {
        &self.docs
    }
}

impl<'a> ReflectStruct<'a> {
    /// Access the metadata associated with this struct definition.
    pub fn meta(&self) -> &ReflectMeta<'a> {
        &self.meta
    }

    /// Access the data about which fields should be ignored during serialization.
    ///
    /// The returned bitset is a collection of indices obtained from the [`members_to_serialization_denylist`](crate::utility::members_to_serialization_denylist) function.
    #[allow(dead_code)]
    pub fn serialization_denylist(&self) -> &BitSet<u32> {
        &self.serialization_denylist
    }

    /// Returns the `GetTypeRegistration` impl as a `TokenStream`.
    ///
    /// Returns a specific implementation for structs and this method should be preferred over the generic [`get_type_registration`](crate::ReflectMeta) method
    pub fn get_type_registration(
        &self,
        where_clause_options: &WhereClauseOptions,
    ) -> proc_macro2::TokenStream {
        crate::registration::impl_get_type_registration(
            self.meta(),
            where_clause_options,
            Some(&self.serialization_denylist),
        )
    }

    /// Get a collection of types which are exposed to the reflection API
    pub fn active_types(&self) -> Vec<syn::Type> {
        self.active_fields()
            .map(|field| field.data.ty.clone())
            .collect()
    }

    /// Get an iterator of fields which are exposed to the reflection API
    pub fn active_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields
            .iter()
            .filter(|field| field.attrs.ignore.is_active())
    }

    /// Get a collection of types which are ignored by the reflection API
    pub fn ignored_types(&self) -> Vec<syn::Type> {
        self.ignored_fields()
            .map(|field| field.data.ty.clone())
            .collect()
    }

    /// Get an iterator of fields which are ignored by the reflection API
    pub fn ignored_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields
            .iter()
            .filter(|field| field.attrs.ignore.is_ignored())
    }

    /// The complete set of fields in this struct.
    #[allow(dead_code)]
    pub fn fields(&self) -> &[StructField<'a>] {
        &self.fields
    }

    pub fn where_clause_options(&self) -> WhereClauseOptions {
        let bevy_reflect_path = &self.meta().bevy_reflect_path;
        WhereClauseOptions {
            active_types: self.active_types().into(),
            active_trait_bounds: quote! { #bevy_reflect_path::Reflect + #bevy_reflect_path::TypePath },
            ignored_types: self.ignored_types().into(),
            ignored_trait_bounds: quote! { #FQAny + #FQSend + #FQSync },
            ..WhereClauseOptions::type_path_bounds(self.meta())
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
        let name = self.meta.path_to_type();
        quote! {
            #name::#variant
        }
    }

    /// The complete set of variants in this enum.
    pub fn variants(&self) -> &[EnumVariant<'a>] {
        &self.variants
    }

    /// Get an iterator of fields which are exposed to the reflection API
    pub fn active_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.variants()
            .iter()
            .flat_map(|variant| variant.active_fields())
    }

    /// Get a collection of types which are exposed to the reflection API
    pub fn active_types(&self) -> Vec<syn::Type> {
        self.active_fields()
            .map(|field| field.data.ty.clone())
            .collect()
    }

    /// Get an iterator of fields which are ignored by the reflection API
    pub fn ignored_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.variants()
            .iter()
            .flat_map(|variant| variant.ignored_fields())
    }

    /// Get a collection of types which are ignored to the reflection API
    pub fn ignored_types(&self) -> Vec<syn::Type> {
        self.ignored_fields()
            .map(|field| field.data.ty.clone())
            .collect()
    }

    pub fn where_clause_options(&self) -> WhereClauseOptions {
        let bevy_reflect_path = &self.meta().bevy_reflect_path;
        WhereClauseOptions {
            active_types: self.active_types().into(),
            active_trait_bounds: quote! { #bevy_reflect_path::FromReflect + #bevy_reflect_path::TypePath },
            ignored_types: self.ignored_types().into(),
            ignored_trait_bounds: quote! { #FQAny + #FQSend + #FQSync + #FQDefault },
            ..WhereClauseOptions::type_path_bounds(self.meta())
        }
    }
}

impl<'a> EnumVariant<'a> {
    /// Get an iterator of fields which are exposed to the reflection API
    #[allow(dead_code)]
    pub fn active_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields()
            .iter()
            .filter(|field| field.attrs.ignore.is_active())
    }

    /// Get an iterator of fields which are ignored by the reflection API
    #[allow(dead_code)]
    pub fn ignored_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields()
            .iter()
            .filter(|field| field.attrs.ignore.is_ignored())
    }

    /// The complete set of fields in this variant.
    #[allow(dead_code)]
    pub fn fields(&self) -> &[StructField<'a>] {
        match &self.fields {
            EnumVariantFields::Named(fields) | EnumVariantFields::Unnamed(fields) => fields,
            EnumVariantFields::Unit => &[],
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
/// The [`non_generic_type_path`], [`non_generic_short_path`],
/// [`type_ident`], [`crate_name`], and [`module_path`] methods
/// are equivalent to the methods on the `TypePath` trait.
///
/// [deriving `TypePath`]: crate::derive_type_path
/// [`non_generic_type_path`]: ReflectTypePath::non_generic_type_path
/// [`non_generic_short_path`]: ReflectTypePath::non_generic_short_path
/// [`type_ident`]: ReflectTypePath::type_ident
/// [`crate_name`]: ReflectTypePath::crate_name
/// [`module_path`]: ReflectTypePath::module_path
///
/// # Example
///
/// ```rust,ignore
/// # use syn::parse_quote;
/// # use bevy_reflect_derive::ReflectTypePath;
///
/// let path: syn::Path = parse_quote!(::core::marker::PhantomData)?;
///
/// let type_path = ReflectTypePath::External {
///     path,
///     custom_path: None,
/// };
///
/// // Eqivalent to "core::marker".
/// let module_path = type_path.module_path();
///
/// # Ok::<(), syn::Error>(())
/// ```
///
pub(crate) enum ReflectTypePath<'a> {
    /// Types without a crate/module that can be named from any scope (e.g. `bool`).
    Primitive(&'a Ident),
    /// Using `::my_crate::foo::Bar` syntax.
    ///
    /// May have a seperate custom path used for the `TypePath` implementation.
    External {
        path: &'a Path,
        custom_path: Option<Path>,
    },
    /// The name of a type relative to its scope.
    ///
    /// The type must be able to be reached with just its name.
    ///
    /// May have a seperate alias path used for the `TypePath` implementation.
    ///
    /// Module and crate are found with [`module_path!()`](core::module_path),
    /// if there is no custom path specified.
    Internal {
        ident: &'a Ident,
        custom_path: Option<Path>,
    },
    /// Any [`syn::Type`] with only a defined `type_path` and `short_type_path`.
    #[allow(dead_code)]
    // Not currently used but may be useful in the future due to its generality.
    Anonymous {
        qualified_type: Type,
        long_type_path: proc_macro2::TokenStream,
        short_type_path: proc_macro2::TokenStream,
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
            Self::Internal { ident, custom_path } => Some(
                custom_path
                    .as_ref()
                    .map(|path| &path.segments.last().unwrap().ident)
                    .unwrap_or(ident),
            ),
            Self::External { path, custom_path } => Some(
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
            Self::External { path, custom_path } => Some(custom_path.as_ref().unwrap_or(path)),
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

    /// Returns tokens for a `&str` representing the name of the type's crate.
    ///
    /// Returns [`None`] if the type is [primitive] or [anonymous].
    ///
    /// For non-aliased [internal] paths this is created from [`module_path`].
    ///
    /// For `::core::option::Option`, this is `"core"`.
    ///
    /// [primitive]: ReflectTypePath::Primitive
    /// [anonymous]: ReflectTypePath::Anonymous
    /// [internal]: ReflectTypePath::Internal
    pub fn crate_name(&self) -> Option<proc_macro2::TokenStream> {
        if let Some(path) = self.get_path() {
            let crate_name = path.segments.first().unwrap().ident.to_string();
            let crate_name = LitStr::new(&crate_name, path.span());
            return Some(quote!(#crate_name));
        }

        match self {
            Self::Internal { .. } => Some(quote! {
                ::core::module_path!()
                    .split(':')
                    .next()
                    .unwrap()
            }),
            Self::External { .. } => unreachable!(),
            _ => None,
        }
    }

    /// Returns tokens for a `&str` representing the "type path" of the type.
    ///
    /// Does not take generics into account.
    ///
    /// For `::core::option::Option`, this is `"core::option::Option"`.
    pub fn non_generic_type_path(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Primitive(ident) => {
                let lit = LitStr::new(&ident.to_string(), ident.span());
                lit.to_token_stream()
            }
            Self::Anonymous { long_type_path, .. } => long_type_path.clone(),
            _ => {
                let Some(ident) = self.get_ident() else {
                    unreachable!()
                };

                let module_path = self.module_path();
                let ident = LitStr::new(&ident.to_string(), ident.span());
                quote! {
                    ::core::concat!(#module_path, "::", #ident)
                }
            }
        }
    }

    /// Returns tokens for a `&str` representing the "short path" of the type.
    ///
    /// Does not take generics into account.
    ///
    /// For `::core::option::Option`, this is `"Option"`.
    pub fn non_generic_short_path(&self) -> proc_macro2::TokenStream {
        if let Some(ident) = self.get_ident() {
            let ident = LitStr::new(&ident.to_string(), ident.span());
            return ident.to_token_stream();
        }

        match self {
            Self::Anonymous {
                short_type_path, ..
            } => short_type_path.clone(),
            _ => unreachable!(),
        }
    }

    /// Returns tokens for a string literal representing the path to the module
    /// that the type is in.
    ///
    /// Returns [`None`] if the type is [primitive] or [anonymous].
    ///
    /// For non-aliased [internal] paths this is created from [`module_path`].
    ///
    /// For `::core::option::Option`, this is `"core::option"`.
    ///
    /// [primitive]: ReflectTypePath::Primitive
    /// [anonymous]: ReflectTypePath::Anonymous
    /// [internal]: ReflectTypePath::Internal
    pub fn module_path(&self) -> Option<proc_macro2::TokenStream> {
        if let Some(path) = self.get_path() {
            let path = path
                .segments
                .pairs()
                .take(path.segments.len() - 1)
                .map(|pair| pair.value().ident.to_string())
                .reduce(|path, ident| path + "::" + &ident)
                .unwrap();

            let path = LitStr::new(&path, path.span());
            return Some(quote! {
                #path
            });
        }

        match self {
            Self::Internal { .. } => Some(quote! {
                ::core::module_path!()
            }),
            Self::External { .. } => unreachable!(),
            _ => None,
        }
    }

    /// Returns tokens for a string literal representing the type's final ident.
    ///
    /// Returns [`None`] if the type is [anonymous].
    ///
    /// This is not necessarily a valid qualified path to the type.
    ///
    /// For `::core::option::Option`, this is `"Option"`.
    ///
    /// [anonymous]: ReflectTypePath::Anonymous
    pub fn type_ident(&self) -> Option<proc_macro2::TokenStream> {
        self.get_ident().map(|ident| {
            let ident = LitStr::new(&ident.to_string(), ident.span());
            ident.to_token_stream()
        })
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
