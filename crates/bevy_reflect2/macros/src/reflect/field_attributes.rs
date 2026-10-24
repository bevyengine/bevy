use bevy_macro_utils::fq_std::FQOption;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parenthesized, spanned::Spanned, token::Paren, Expr, Field, Path, Result};

use crate::reflect::{attributes::TraitImpl, REFLECT_ATTRIBUTE_NAME};

const IGNORE: &str = "ignore";
const DEFAULT: &str = "default";
const CLONE: &str = "clone";

/// Attributes on a field, from `#[reflect(...)]`.
///
/// Examples:
/// - `#[reflect(ignore)]`
/// - `#[reflect(ignore, default, clone)]`
/// - `#[reflect(ignore, default(custom_default_fn), clone(|value| value.clone()))]`
#[derive(Default)]
pub(crate) struct FieldAttributes {
    /// Excludes the field from reflection.
    pub(crate) ignore: bool,
    /// Initializes an ignored field when the value is constructed field-by-field.
    pub(crate) default: TraitImpl,
    /// Clones an ignored field when the value is cloned structurally.
    pub(crate) clone: TraitImpl,
}

impl FieldAttributes {
    /// Parses the attributes of `field`.
    pub(crate) fn parse(field: &Field) -> Result<Self> {
        let mut attributes = Self::default();
        for attr in &field.attrs {
            if !attr.path().is_ident(REFLECT_ATTRIBUTE_NAME) {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                let span = meta.path.span();
                // Parses the optional custom function in `trait(custom_fn)`
                let trait_impl = || -> Result<TraitImpl> {
                    if meta.input.peek(Paren) {
                        let content;
                        parenthesized!(content in meta.input);
                        Ok(TraitImpl::Custom(Box::new(content.parse::<Expr>()?), span))
                    } else {
                        Ok(TraitImpl::Implemented(span))
                    }
                };
                if meta.path.is_ident(IGNORE) {
                    if attributes.ignore {
                        return Err(meta.error(format_args!("`{IGNORE}` is specified more than once")));
                    }
                    attributes.ignore = true;
                    Ok(())
                } else if meta.path.is_ident(DEFAULT) {
                    attributes.default.merge(trait_impl()?)
                } else if meta.path.is_ident(CLONE) {
                    attributes.clone.merge(trait_impl()?)
                } else {
                    Err(meta.error(format_args!(
                        "unsupported field attribute, expected `{IGNORE}`, `{DEFAULT}`, or `{CLONE}`"
                    )))
                }
            })?;
        }

        if !attributes.ignore {
            for (name, trait_impl) in [(DEFAULT, &attributes.default), (CLONE, &attributes.clone)] {
                if let TraitImpl::Implemented(span) | TraitImpl::Custom(_, span) = trait_impl {
                    return Err(syn::Error::new(
                        *span,
                        format_args!(
                            "`{name}` is only supported on ignored fields (`#[{REFLECT_ATTRIBUTE_NAME}({IGNORE}, {name})]`)"
                        ),
                    ));
                }
            }
        }
        Ok(attributes)
    }

    /// Returns an expression that creates the default value of an ignored field, or `None` if it
    /// has no default.
    pub(crate) fn default_value(&self, field: &Field, bevy_reflect: &Path) -> Option<TokenStream> {
        let field_ty = &field.ty;
        match &self.default {
            &TraitImpl::Implemented(span) => Some(quote_spanned! {span=>
                <#field_ty as ::core::default::Default>::default()
            }),
            &TraitImpl::Custom(ref impl_fn, span) => Some(quote_spanned! {span=>
                #bevy_reflect::__macro_exports::custom_default::<#field_ty>(#impl_fn)
            }),
            TraitImpl::NotImplemented => None,
        }
    }

    /// Returns the `IgnoredFieldVTable` for an ignored field of type `field_ty`.
    pub(crate) fn ignored_field_vtable(&self, field: &Field, bevy_reflect: &Path) -> TokenStream {
        let field_ty = &field.ty;
        let exports = quote! {#bevy_reflect::__macro_exports};
        let default = match &self.default {
            &TraitImpl::Implemented(span) => quote_spanned! {span=>
                #FQOption::Some(#bevy_reflect::set_default::<#field_ty>)
            },
            &TraitImpl::Custom(ref impl_fn, span) => quote_spanned! {span=>
                #FQOption::Some(
                    (|ptr: *mut u8| {
                        // SAFETY: only called through `IgnoredFieldVTable::default`, whose caller
                        // guarantees that `ptr` is a valid, unique allocation of the field's type
                        unsafe { #bevy_reflect::set_with::<#field_ty>(ptr, #impl_fn) }
                    }) as unsafe fn(*mut u8)
                )
            },
            TraitImpl::NotImplemented => quote! {#FQOption::None},
        };
        let clone = match &self.clone {
            &TraitImpl::Implemented(span) => quote_spanned! {span=>
                #FQOption::Some(#bevy_reflect::clone_into::<#field_ty>)
            },
            &TraitImpl::Custom(ref impl_fn, span) => quote_spanned! {span=>
                #FQOption::Some(
                    (|src: #exports::Ptr<'_>, dst: *mut u8| {
                        // SAFETY: only called through `IgnoredFieldVTable::clone`, whose caller
                        // guarantees that `src` is a valid value of the field's type, and that `dst`
                        // is a valid, unique allocation of it
                        unsafe { #bevy_reflect::clone_into_with::<#field_ty>(src, dst, #impl_fn) }
                    }) as unsafe fn(#exports::Ptr<'_>, *mut u8)
                )
            },
            TraitImpl::NotImplemented => quote! {#FQOption::None},
        };
        quote! {
            #bevy_reflect::IgnoredFieldVTable {
                layout: ::core::alloc::Layout::new::<#field_ty>(),
                default: #default,
                clone: #clone,
            }
        }
    }
}
