use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    spanned::Spanned, Data, DeriveInput, Field, Fields, Ident, Member, Path, Result, Type, Variant,
};

use crate::reflect::field_attributes::FieldAttributes;

/// The fields or variants of a reflected type, parsed once from its definition.
pub(crate) enum ReflectData<'a> {
    Struct(ReflectFields<'a>),
    Enum(Vec<ReflectVariant<'a>>),
}

impl<'a> ReflectData<'a> {
    pub(crate) fn parse(ast: &'a DeriveInput) -> Result<Self> {
        match &ast.data {
            Data::Struct(data_struct) => {
                Ok(Self::Struct(ReflectFields::parse(&data_struct.fields)?))
            }
            Data::Enum(data_enum) => Ok(Self::Enum(
                data_enum
                    .variants
                    .iter()
                    .map(ReflectVariant::parse)
                    .collect::<Result<_>>()?,
            )),
            Data::Union(data_union) => Err(syn::Error::new(
                data_union.union_token.span(),
                "`Reflect` cannot be derived for unions",
            )),
        }
    }

    /// Returns every field of the struct, or of every variant of the enum, in declaration order.
    pub(crate) fn fields(&self) -> impl Iterator<Item = &ReflectField<'a>> {
        let fields: Vec<&ReflectFields<'a>> = match self {
            Self::Struct(fields) => vec![fields],
            Self::Enum(variants) => variants.iter().map(|variant| &variant.fields).collect(),
        };
        fields.into_iter().flat_map(|fields| &fields.fields)
    }

    /// Returns the types of the reflected (non-ignored) fields, in declaration order.
    pub(crate) fn reflected_field_types(&self) -> Vec<&Type> {
        self.fields()
            .filter(|field| !field.attributes.ignore)
            .map(|field| &field.field.ty)
            .collect()
    }
}

/// An enum variant of a reflected type.
pub(crate) struct ReflectVariant<'a> {
    pub(crate) variant: &'a Variant,
    pub(crate) fields: ReflectFields<'a>,
}

impl<'a> ReflectVariant<'a> {
    fn parse(variant: &'a Variant) -> Result<Self> {
        Ok(Self {
            variant,
            fields: ReflectFields::parse(&variant.fields)?,
        })
    }
}

/// The fields of a struct or enum variant.
pub(crate) struct ReflectFields<'a> {
    pub(crate) definition: &'a Fields,
    pub(crate) fields: Vec<ReflectField<'a>>,
}

impl<'a> ReflectFields<'a> {
    /// Parses the attributes of each field.
    fn parse(definition: &'a Fields) -> Result<Self> {
        let fields = definition
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let (member, binding) = match &field.ident {
                    // Bindings are prefixed, so that they don't shadow names used by the
                    // generated code (ex: an `index` parameter)
                    Some(ident) => (
                        Member::Named(ident.clone()),
                        format_ident!("__field_{}", ident),
                    ),
                    None => (Member::Unnamed(index.into()), format_ident!("f{}", index)),
                };
                Ok(ReflectField {
                    field,
                    member,
                    binding,
                    attributes: FieldAttributes::parse(field)?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self { definition, fields })
    }

    pub(crate) fn field_kind(&self, bevy_reflect: &Path) -> TokenStream {
        match self.definition {
            Fields::Named(_) => quote! { #bevy_reflect::FieldKind::Named },
            Fields::Unnamed(_) => quote! { #bevy_reflect::FieldKind::Unnamed },
            Fields::Unit => quote! { #bevy_reflect::FieldKind::Unit },
        }
    }
}

/// A field of a struct or enum variant, with its parsed attributes.
pub(crate) struct ReflectField<'a> {
    pub(crate) field: &'a Field,
    /// The member used to access the field (ex: `value` or `0`).
    pub(crate) member: Member,
    /// The identifier used to bind the field in patterns (ex: `__field_value` or `f0`).
    pub(crate) binding: Ident,
    pub(crate) attributes: FieldAttributes,
}

impl ReflectField<'_> {
    /// Returns the name of the field in reflected type information (ex: `value` or `0`).
    pub(crate) fn name(&self) -> String {
        match &self.member {
            Member::Named(ident) => ident.to_string(),
            Member::Unnamed(index) => index.index.to_string(),
        }
    }

    /// Returns the pattern that binds the field to [`binding`](Self::binding) (ex:
    /// `value: __field_value` or `0: f0`), for use in braced patterns like `Self { .. }`.
    pub(crate) fn pattern(&self) -> TokenStream {
        let binding = &self.binding;
        let member = &self.member;
        quote! {#member: #binding}
    }
}
