use bevy_macro_utils::BevyManifest;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::quote;
use syn::{
    parse::{ParseStream, Parser},
    spanned::Spanned,
    Attribute, Generics, Ident, LitStr, Path, Token, Type,
};

use crate::reflect::{
    attributes::ContainerAttributes,
    extend_where_clause, take_type_path_attribute,
    type_path::{type_path, type_path_strings},
    type_registration::type_registration,
};

const REFLECT_ATTRIBUTE_NAME: &str = "reflect";

mod kw {
    syn::custom_keyword!(primitive);
}

/// Implements reflection for a type as a `Kind::Primitive`, whose internals are not reflected.
///
/// Supports two forms:
/// - `::path::to::Type<T: Bound>(Trait, Trait(custom_fn), TypeData)`: the path determines the type
///   path. Single-segment paths (ex: `u32`) have no crate or module.
/// - `#[type_path = "path::to::Type"] SomeType(Trait)`: for non-generic types whose type path cannot be
///   inferred from their syntax (ex: `&'static str`).
///
/// Both forms accept an optional `#[reflect(primitive = Variant)]` attribute, which selects the `Primitive`
/// variant. It defaults to `Primitive::Opaque`.
pub(crate) fn impl_reflect_primitive(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let bevy_reflect = BevyManifest::shared(|manifest| manifest.get_path("bevy_reflect2"));

    // The trailing parenthesized group contains the reflected traits. Splitting it off first prevents
    // it from being parsed as part of the type (ex: `str(Clone)` looks like `Fn(Clone)` sugar).
    let mut tokens = input.into_iter().collect::<Vec<_>>();
    let traits = match tokens.pop() {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
            group.stream()
        }
        Some(token) => {
            tokens.push(token);
            TokenStream::new()
        }
        None => TokenStream::new(),
    };
    let mut container_attributes = ContainerAttributes::default();
    Parser::parse2(
        |input: ParseStream| container_attributes.parse_terminated(input),
        traits,
    )?;

    let (mut attrs, rest) = Parser::parse2(
        |input: ParseStream| {
            Ok((
                Attribute::parse_outer(input)?,
                input.parse::<TokenStream>()?,
            ))
        },
        tokens.into_iter().collect(),
    )?;

    let primitive = take_primitive_attribute(&mut attrs)?
        .unwrap_or_else(|| Ident::new("Opaque", proc_macro2::Span::call_site()));

    let (target, generics, type_path_tokens, generic_type_path_impl, extended_where_clause) =
        match take_type_path_attribute(&mut attrs)? {
            Some(full_path) => {
                let ty = syn::parse2::<Type>(rest)?;
                let type_path = explicit_type_path(&bevy_reflect, &full_path)?;
                (
                    quote! {#ty},
                    Generics::default(),
                    type_path,
                    quote! {},
                    quote! {},
                )
            }
            None => {
                let (path, generics) = Parser::parse2(
                    |input: ParseStream| {
                        Ok((Path::parse_mod_style(input)?, input.parse::<Generics>()?))
                    },
                    rest,
                )?;
                let mut segments = path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string());
                let type_ident_str = segments
                    .next_back()
                    .ok_or_else(|| syn::Error::new_spanned(&path, "expected a path to a type"))?;
                let module_path = segments.collect::<Vec<_>>().join("::");
                if module_path.is_empty() {
                    // Types without a module (ex: `u32`) have no crate name or module path.
                    if !generics.params.is_empty() {
                        return Err(syn::Error::new_spanned(
                            &generics,
                            "generic types must be specified using a path that starts with the crate name",
                        ));
                    }
                    let full_path = LitStr::new(&type_ident_str, path.span());
                    let type_path = explicit_type_path(&bevy_reflect, &full_path)?;
                    (quote! {#path}, generics, type_path, quote! {}, quote! {})
                } else {
                    let module_path = LitStr::new(&module_path, proc_macro2::Span::call_site());
                    let module_path = quote! {#module_path};

                    let (_, type_generics, _) = generics.split_for_impl();
                    let target = quote! {#path #type_generics};
                    let extended_where_clause =
                        extend_where_clause(&generics, quote! {#bevy_reflect::Typed}, &[]);
                    let (generic_type_path_impl, full_type_path, short_type_path) =
                        type_path_strings(
                            &target,
                            &generics,
                            &bevy_reflect,
                            &extended_where_clause,
                            &type_ident_str,
                            &module_path,
                            &[],
                        );
                    let type_path = type_path(
                        &bevy_reflect,
                        &module_path,
                        &full_type_path,
                        &short_type_path,
                        type_ident_str.len(),
                    );
                    (
                        target,
                        generics,
                        type_path,
                        generic_type_path_impl,
                        extended_where_clause,
                    )
                }
            }
        };
    if let Some(attr) = attrs.first() {
        return Err(syn::Error::new_spanned(attr, "unsupported attribute"));
    }

    let (impl_generics, _, _) = generics.split_for_impl();
    let vtable = container_attributes.vtable(&bevy_reflect);
    let type_registration = type_registration(
        &target,
        &generics,
        &bevy_reflect,
        &extended_where_clause,
        &container_attributes.type_data,
        &[],
    );
    // SAFETY: primitive types have no reflected fields, and the layout and vtable are derived from `Self`
    Ok(quote! {
        unsafe impl #impl_generics #bevy_reflect::Typed for #target #extended_where_clause {
            const TYPE: &'static #bevy_reflect::Type = &{
                let vtable = #vtable;
                unsafe {
                    #bevy_reflect::Type::new(
                        ::core::any::TypeId::of::<Self>(),
                        #bevy_reflect::Kind::Primitive(#bevy_reflect::Primitive::#primitive),
                        Some(::core::alloc::Layout::new::<Self>()),
                        vtable,
                        #type_path_tokens,
                    )
                }
            };
        }

        #generic_type_path_impl
        #type_registration
    })
}

/// Removes the `#[reflect(primitive = Variant)]` attributes from `attrs` and returns the variant (if
/// one is specified). Traits and type data are registered in the trailing parentheses instead, so
/// `primitive` is the only option supported by these attributes.
fn take_primitive_attribute(attrs: &mut Vec<Attribute>) -> Result<Option<Ident>, syn::Error> {
    let mut primitive = None;
    let mut result = Ok(());
    attrs.retain(|attr| {
        if !attr.path().is_ident(REFLECT_ATTRIBUTE_NAME) {
            return true;
        }
        let parsed = attr.parse_args_with(|input: ParseStream| {
            let keyword = input.parse::<kw::primitive>().map_err(|err| {
                syn::Error::new(
                    err.span(),
                    "expected `primitive = Variant` (traits and type data are registered in the trailing parentheses)",
                )
            })?;
            input.parse::<Token![=]>()?;
            let variant = input.parse::<Ident>()?;
            input.parse::<Option<Token![,]>>()?;
            Ok((keyword, variant))
        });
        let parsed = parsed.and_then(|(keyword, variant)| {
            if primitive.is_some() {
                return Err(syn::Error::new(
                    keyword.span,
                    "the primitive variant is already specified",
                ));
            }
            primitive = Some(variant);
            Ok(())
        });
        if let Err(err) = parsed {
            match &mut result {
                Ok(()) => result = Err(err),
                Err(existing) => existing.combine(err),
            }
        }
        false
    });
    result.map(|()| primitive)
}

/// Returns a static `TypePath` for a full type path string, such as `alloc::borrow::Cow<str>`.
///
/// Leading `&` characters are kept in the full and short paths, but are excluded from the type
/// identifier, crate name, and module path (ex: `&std::path::Path` has the short path `&Path`).
/// Paths without a module (ex: `&str`) have no crate name or module path.
fn explicit_type_path(bevy_reflect: &Path, full_path: &LitStr) -> Result<TokenStream, syn::Error> {
    let full = full_path.value();
    let prefix_len = full.len() - full.trim_start_matches('&').len();
    let generics_start = full.find('<').unwrap_or(full.len());
    if prefix_len >= generics_start {
        return Err(syn::Error::new(full_path.span(), "expected a type path"));
    }
    let prefix = &full[..prefix_len];
    let (short, crate_name, module_path) = match full[..generics_start].rfind("::") {
        Some(module_end) => {
            let crate_end = full.find("::").unwrap_or(module_end);
            (
                format!("{prefix}{}", &full[module_end + 2..]),
                quote! {Some(#prefix_len..#crate_end)},
                quote! {Some(#prefix_len..#module_end)},
            )
        }
        None => (full.clone(), quote! {None}, quote! {None}),
    };
    let type_ident_end = prefix_len
        + short[prefix_len..]
            .find('<')
            .unwrap_or(short.len() - prefix_len);
    Ok(quote! {
        #bevy_reflect::TypePath::new(
            #bevy_reflect::TypePathStr::Static(#full),
            #bevy_reflect::TypePathStr::Static(#short),
            #prefix_len..#type_ident_end,
            #crate_name,
            #module_path,
        )
    })
}
