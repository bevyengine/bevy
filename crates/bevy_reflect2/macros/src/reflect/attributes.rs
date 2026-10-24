use bevy_macro_utils::{fq_std::FQOption, terminated_parser};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    ext::IdentExt, parenthesized, parse::ParseStream, punctuated::Punctuated, spanned::Spanned,
    token::Paren, Attribute, Expr, Ident, MetaList, Path, Result, Token, WherePredicate,
};

use crate::reflect::type_data::TypeDataRegistration;

#[derive(Default)]
pub(crate) struct ContainerAttributes {
    pub(crate) default: TraitImpl,
    pub(crate) clone: TraitImpl,
    pub(crate) debug: TraitImpl,
    pub(crate) hash: TraitImpl,
    pub(crate) partial_eq: TraitImpl,
    pub(crate) partial_ord: TraitImpl,
    // custom_attributes: CustomAttributes,
    pub(crate) type_data: Vec<TypeDataRegistration>,
    pub(crate) no_auto_register: bool,
    /// Reflects the type as an [`Primitive::Opaque`], without reflecting its fields.
    pub(crate) opaque: bool,
    /// Additional where clause predicates for the generated impls (`#[reflect(where T: Bound)]`).
    pub(crate) where_predicates: Vec<WherePredicate>,
    /// Type parameters that are not required to be `Typed` (`#[reflect(ignore_params(F))]`).
    pub(crate) ignore_params: Vec<Ident>,
}

impl ContainerAttributes {
    /// Parse a comma-separated list of container attributes.
    ///
    /// # Example
    /// - `Hash, Debug(custom_debug), MyTrait`
    pub fn parse_terminated(&mut self, input: ParseStream) -> Result<()> {
        terminated_parser(Token![,], |stream| self.parse_container_attribute(stream))(input)?;

        Ok(())
    }

    /// Parse the contents of a `#[reflect(...)]` attribute into a [`ContainerAttributes`] instance.
    ///
    /// # Example
    /// - `#[reflect(Hash, Debug(custom_debug), MyTrait)]`
    /// - `#[reflect(no_field_bounds)]`
    pub fn parse_meta_list(&mut self, meta: &MetaList) -> Result<()> {
        meta.parse_args_with(|stream: ParseStream| self.parse_terminated(stream))
    }

    /// Parse a single container attribute.
    fn parse_container_attribute(&mut self, input: ParseStream) -> Result<()> {
        if input.peek(Token![#]) {
            return self.parse_cfg_type_data(input);
        }
        if input.peek(Token![where]) {
            return self.parse_where(input);
        }
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::Default) {
            let span = input.parse::<kw::Default>()?.span;
            Self::parse_trait_impl(&mut self.default, span, input)
        } else if lookahead.peek(kw::Clone) {
            let span = input.parse::<kw::Clone>()?.span;
            Self::parse_trait_impl(&mut self.clone, span, input)
        } else if lookahead.peek(kw::Debug) {
            let span = input.parse::<kw::Debug>()?.span;
            Self::parse_trait_impl(&mut self.debug, span, input)
        } else if lookahead.peek(kw::Hash) {
            let span = input.parse::<kw::Hash>()?.span;
            Self::parse_trait_impl(&mut self.hash, span, input)
        } else if lookahead.peek(kw::PartialEq) {
            let span = input.parse::<kw::PartialEq>()?.span;
            Self::parse_trait_impl(&mut self.partial_eq, span, input)
        } else if lookahead.peek(kw::PartialOrd) {
            let span = input.parse::<kw::PartialOrd>()?.span;
            Self::parse_trait_impl(&mut self.partial_ord, span, input)
        } else if lookahead.peek(kw::no_auto_register) {
            self.parse_no_auto_register(input)
        } else if lookahead.peek(kw::opaque) {
            input.parse::<kw::opaque>()?;
            self.opaque = true;
            Ok(())
        } else if lookahead.peek(kw::ignore_params) {
            input.parse::<kw::ignore_params>()?;
            let content;
            parenthesized!(content in input);
            let params = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
            self.ignore_params.extend(params);
            Ok(())
        } else if lookahead.peek(Ident::peek_any) {
            self.parse_type_data(input)
        } else {
            Err(lookahead.error())
        }
    }
    /// Parse a type data registration that is conditionally enabled by `#[cfg(...)]` attributes.
    ///
    /// Examples:
    /// - `#[cfg(feature = "std")] Default`
    fn parse_cfg_type_data(&mut self, input: ParseStream) -> Result<()> {
        let cfg_attrs = input.call(Attribute::parse_outer)?;
        if let Some(attr) = cfg_attrs.iter().find(|attr| !attr.path().is_ident("cfg")) {
            return Err(syn::Error::new_spanned(
                attr,
                "only `#[cfg(...)]` attributes are supported here",
            ));
        }
        if input.peek(kw::Default)
            || input.peek(kw::Clone)
            || input.peek(kw::Debug)
            || input.peek(kw::Hash)
            || input.peek(kw::PartialEq)
            || input.peek(kw::PartialOrd)
            || input.peek(kw::no_auto_register)
            || input.peek(kw::opaque)
            || input.peek(kw::ignore_params)
        {
            return Err(input.error("`#[cfg(...)]` is only supported on type data registrations"));
        }
        let start = self.type_data.len();
        self.parse_type_data(input)?;
        if let Some(type_data) = self.type_data.get_mut(start) {
            type_data.set_cfg_attrs(cfg_attrs);
        }
        Ok(())
    }

    ///
    /// Parse a type data registration.
    ///
    /// Examples:
    /// - `#[reflect(Default)]`
    /// - `#[reflect(Hash(custom_hash_fn))]`
    fn parse_type_data(&mut self, input: ParseStream) -> Result<()> {
        let type_data = input.parse::<TypeDataRegistration>()?;

        if self
            .type_data
            .iter()
            .any(|existing| existing.path() == type_data.path())
        {
            return Err(syn::Error::new(
                type_data.path().span(),
                CONFLICTING_TYPE_DATA_MESSAGE,
            ));
        }

        self.type_data.push(type_data);

        Ok(())
    }

    /// Parse a built-in trait attribute (the trait keyword has already been parsed).
    ///
    /// Examples:
    /// - `#[reflect(Default)]`
    /// - `#[reflect(Debug(custom_debug_fn))]`
    /// - `#[reflect(Clone(|value: &Foo| Foo(value.0)))]`
    fn parse_trait_impl(trait_impl: &mut TraitImpl, span: Span, input: ParseStream) -> Result<()> {
        if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            let impl_fn = content.parse::<Expr>()?;
            trait_impl.merge(TraitImpl::Custom(Box::new(impl_fn), span))
        } else {
            trait_impl.merge(TraitImpl::Implemented(span))
        }
    }

    /// Parse additional where clause predicates. These consume the rest of the attribute, so they
    /// must come last.
    ///
    /// Examples:
    /// - `#[reflect(where T::Assoc: GetTypeRegistration)]`
    fn parse_where(&mut self, input: ParseStream) -> Result<()> {
        input.parse::<Token![where]>()?;
        let predicates = Punctuated::<WherePredicate, Token![,]>::parse_terminated(input)?;
        self.where_predicates.extend(predicates);
        Ok(())
    }

    fn parse_no_auto_register(&mut self, input: ParseStream) -> Result<()> {
        input.parse::<kw::no_auto_register>()?;
        self.no_auto_register = true;
        Ok(())
    }

    pub fn default_impl(&self, bevy_reflect: &Path) -> TokenStream {
        match &self.default {
            &TraitImpl::Implemented(span) => quote_spanned! {span=>
                #FQOption::Some(#bevy_reflect::set_default::<Self>)
            },
            &TraitImpl::Custom(ref impl_fn, span) => quote_spanned! {span=>
                #FQOption::Some(
                    (|ptr: *mut u8| {
                        // SAFETY: only called through `TypeVTable::set_default`, whose caller
                        // guarantees that `ptr` is a valid, unique allocation of `Self`
                        unsafe { #bevy_reflect::set_with::<Self>(ptr, #impl_fn) }
                    }) as unsafe fn(*mut u8)
                )
            },
            TraitImpl::NotImplemented => {
                quote! {#FQOption::None}
            }
        }
    }

    pub fn clone_impl(&self, bevy_reflect: &Path) -> TokenStream {
        let exports = quote! {#bevy_reflect::__macro_exports};
        let clone_fn = quote! {
            unsafe fn(#exports::Ptr<'_>) -> #FQOption<#exports::Box<dyn #bevy_reflect::Reflect>>
        };
        match &self.clone {
            &TraitImpl::Implemented(span) => quote_spanned! {span=>
                #FQOption::Some(#bevy_reflect::clone::<Self>)
            },
            // SAFETY (all cases below): only called through `TypeVTable::clone`, whose caller
            // guarantees that `ptr` is a valid `Self`
            &TraitImpl::Custom(ref impl_fn, span) => quote_spanned! {span=>
                #FQOption::Some(
                    (|ptr: #exports::Ptr<'_>| {
                        unsafe { #bevy_reflect::clone_with::<Self>(ptr, #exports::custom_clone::<Self>(#impl_fn)) }
                    }) as #clone_fn
                )
            },
            TraitImpl::NotImplemented => quote! {#FQOption::None},
        }
    }

    pub fn debug_impl(&self, bevy_reflect: &Path) -> TokenStream {
        let exports = quote! {#bevy_reflect::__macro_exports};
        match &self.debug {
            &TraitImpl::Implemented(span) => quote_spanned! {span=>
                #FQOption::Some(#bevy_reflect::debug::<Self>)
            },
            &TraitImpl::Custom(ref impl_fn, span) => {
                // SAFETY: only called through `TypeVTable::debug`, whose caller guarantees
                // that `ptr` is a valid `Self`
                quote_spanned! {span=>
                    #FQOption::Some(
                        (|ptr: #exports::Ptr<'_>, f: &mut ::core::fmt::Formatter<'_>| {
                            unsafe { #bevy_reflect::debug_with::<Self>(ptr, f, #impl_fn) }
                        }) as unsafe fn(#exports::Ptr<'_>, &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result
                    )
                }
            }
            TraitImpl::NotImplemented => quote! {#FQOption::None},
        }
    }

    pub fn hash_impl(&self, bevy_reflect: &Path) -> TokenStream {
        let exports = quote! {#bevy_reflect::__macro_exports};
        match &self.hash {
            &TraitImpl::Implemented(span) => quote_spanned! {span=>
                #FQOption::Some(#bevy_reflect::hash::<Self>)
            },
            &TraitImpl::Custom(ref impl_fn, span) => {
                // SAFETY: only called through `TypeVTable::hash`, whose caller guarantees
                // that `ptr` is a valid `Self`
                quote_spanned! {span=>
                    #FQOption::Some(
                        (|ptr: #exports::Ptr<'_>, hasher: &mut dyn ::core::hash::Hasher| {
                            unsafe { #bevy_reflect::hash_with::<Self>(ptr, hasher, #impl_fn) }
                        }) as unsafe fn(#exports::Ptr<'_>, &mut dyn ::core::hash::Hasher)
                    )
                }
            }
            TraitImpl::NotImplemented => quote! {#FQOption::None},
        }
    }

    pub fn partial_eq_impl(&self, bevy_reflect: &Path) -> TokenStream {
        let exports = quote! {#bevy_reflect::__macro_exports};
        match &self.partial_eq {
            &TraitImpl::Implemented(span) => quote_spanned! {span=>
                #FQOption::Some(#bevy_reflect::partial_eq::<Self>)
            },
            &TraitImpl::Custom(ref impl_fn, span) => {
                // SAFETY: only called through `TypeVTable::partial_eq`, whose caller
                // guarantees that `a` and `b` are valid `Self` values
                quote_spanned! {span=>
                    #FQOption::Some(
                        (|a: #exports::Ptr<'_>, b: #exports::Ptr<'_>| {
                            unsafe { #bevy_reflect::partial_eq_with::<Self>(a, b, #impl_fn) }
                        }) as unsafe fn(#exports::Ptr<'_>, #exports::Ptr<'_>) -> bool
                    )
                }
            }
            TraitImpl::NotImplemented => quote! {#FQOption::None},
        }
    }

    pub fn partial_ord_impl(&self, bevy_reflect: &Path) -> TokenStream {
        let exports = quote! {#bevy_reflect::__macro_exports};
        match &self.partial_ord {
            &TraitImpl::Implemented(span) => quote_spanned! {span=>
                #FQOption::Some(#bevy_reflect::partial_cmp::<Self>)
            },
            &TraitImpl::Custom(ref impl_fn, span) => {
                // SAFETY: only called through `TypeVTable::partial_cmp`, whose caller
                // guarantees that `a` and `b` are valid `Self` values
                quote_spanned! {span=>
                    #FQOption::Some(
                        (|a: #exports::Ptr<'_>, b: #exports::Ptr<'_>| {
                            unsafe { #bevy_reflect::partial_cmp_with::<Self>(a, b, #impl_fn) }
                        }) as unsafe fn(#exports::Ptr<'_>, #exports::Ptr<'_>) -> #FQOption<::core::cmp::Ordering>
                    )
                }
            }
            TraitImpl::NotImplemented => quote! {#FQOption::None},
        }
    }

    /// Returns the `TypeVTable` for `Self`.
    pub fn vtable(&self, bevy_reflect: &Path) -> TokenStream {
        let default = self.default_impl(bevy_reflect);
        let clone = self.clone_impl(bevy_reflect);
        let debug = self.debug_impl(bevy_reflect);
        let hash = self.hash_impl(bevy_reflect);
        let partial_eq = self.partial_eq_impl(bevy_reflect);
        let partial_cmp = self.partial_ord_impl(bevy_reflect);
        quote! {
            #bevy_reflect::TypeVTable {
                cast_reflect: #bevy_reflect::cast_reflect::<Self>,
                set_default: #default,
                clone: #clone,
                debug: #debug,
                hash: #hash,
                partial_eq: #partial_eq,
                partial_cmp: #partial_cmp,
            }
        }
    }
}

// The error message to show when a trait/type is specified multiple times
const CONFLICTING_TYPE_DATA_MESSAGE: &str = "conflicting type data registration";

/// A marker for trait implementations registered via the `Reflect` derive macro.
#[derive(Clone, Default)]
pub(crate) enum TraitImpl {
    /// The trait is not registered as implemented.
    #[default]
    NotImplemented,

    /// The trait is registered as implemented.
    Implemented(Span),

    /// The trait is registered with a custom function (any function expression, including closures)
    /// rather than an actual implementation.
    Custom(Box<Expr>, Span),
}

impl TraitImpl {
    /// Merges this [`TraitImpl`] with another.
    ///
    /// Update `self` with whichever value is not [`TraitImpl::NotImplemented`].
    /// If `other` is [`TraitImpl::NotImplemented`], then `self` is not modified.
    /// An error is returned if neither value is [`TraitImpl::NotImplemented`].
    pub fn merge(&mut self, other: TraitImpl) -> Result<()> {
        match (&self, other) {
            (TraitImpl::NotImplemented, value) => {
                *self = value;
                Ok(())
            }
            (_, TraitImpl::NotImplemented) => Ok(()),
            (_, TraitImpl::Implemented(span) | TraitImpl::Custom(_, span)) => {
                Err(syn::Error::new(span, CONFLICTING_TYPE_DATA_MESSAGE))
            }
        }
    }
}

mod kw {
    syn::custom_keyword!(Debug);
    syn::custom_keyword!(PartialEq);
    syn::custom_keyword!(PartialOrd);
    syn::custom_keyword!(Hash);
    syn::custom_keyword!(Clone);
    syn::custom_keyword!(Default);
    syn::custom_keyword!(no_auto_register);
    syn::custom_keyword!(opaque);
    syn::custom_keyword!(ignore_params);
}
