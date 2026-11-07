use bevy_macro_utils::BevyManifest;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{ParseStream, Parser},
    spanned::Spanned,
    Attribute, Generics, Ident, LitStr, Path, Type,
};

use crate::reflect::{
    extend_where_clause, ignored_params, parse_container_attributes, take_string_attribute,
    type_path::{type_path, type_path_strings},
    type_registration::type_registration,
    FULL_TYPE_PATH_ATTRIBUTE_NAME, REFLECT_ATTRIBUTE_NAME, TYPE_PATH_ATTRIBUTE_NAME,
};

/// Implements reflection for a type as a `Kind::Primitive`, whose internals are not reflected. This
/// is the form of `impl_reflect!` used for types that are not re-stated definitions.
///
/// Supports two forms:
/// - `::path::to::Type<T: Bound>`: the path determines the type path. Single-segment paths (ex:
///   `u32`) have no crate or module.
/// - `#[full_type_path = "path::to::Type"] SomeType`: for non-generic types whose type path cannot
///   be inferred from their syntax (ex: `&'static str`).
///
/// Both forms accept `#[reflect(...)]` container attributes, which register traits and type data.
/// The `Primitive` variant is selected with `#[reflect(primitive = Variant)]`, and defaults to
/// `Primitive::Opaque`.
pub(crate) fn impl_reflect_primitive(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let bevy_reflect = BevyManifest::shared(|manifest| manifest.get_path("bevy_reflect2"));

    let (mut attrs, rest) = Parser::parse2(
        |input: ParseStream| {
            Ok((
                Attribute::parse_outer(input)?,
                input.parse::<TokenStream>()?,
            ))
        },
        input,
    )?;
    let full_type_path = take_string_attribute(&mut attrs, FULL_TYPE_PATH_ATTRIBUTE_NAME)?;
    if let Some(attr) = attrs
        .iter()
        .find(|attr| attr.path().is_ident(TYPE_PATH_ATTRIBUTE_NAME))
    {
        return Err(syn::Error::new_spanned(
            attr,
            format_args!(
                "`#[{TYPE_PATH_ATTRIBUTE_NAME} = \"...\"]` is only supported on re-stated definitions. \
                Types that are not re-stated infer their type path from their syntax, or specify it with \
                `#[{FULL_TYPE_PATH_ATTRIBUTE_NAME} = \"path::to::Type\"]`"
            ),
        ));
    }
    if let Some(attr) = attrs
        .iter()
        .find(|attr| !attr.path().is_ident(REFLECT_ATTRIBUTE_NAME))
    {
        return Err(syn::Error::new_spanned(attr, "unsupported attribute"));
    }
    let container_attributes = parse_container_attributes(&attrs)?;
    let primitive = container_attributes
        .primitive
        .clone()
        .unwrap_or_else(|| Ident::new("Opaque", proc_macro2::Span::call_site()));
    // Custom where clause predicates (`#[reflect(where ...)]`) apply to every generated impl
    let with_where_predicates = |mut generics: Generics| {
        if !container_attributes.where_predicates.is_empty() {
            generics
                .make_where_clause()
                .predicates
                .extend(container_attributes.where_predicates.iter().cloned());
        }
        generics
    };

    let (target, generics, type_path_tokens, generic_type_path_impl, extended_where_clause) =
        match full_type_path {
            Some(full_path) => {
                let ty = syn::parse2::<Type>(rest)?;
                let type_path = explicit_type_path(&bevy_reflect, &full_path)?;
                let generics = with_where_predicates(Generics::default());
                ignored_params(&generics, &container_attributes)?;
                let where_clause = &generics.where_clause;
                let where_clause = quote! {#where_clause};
                (quote! {#ty}, generics, type_path, quote! {}, where_clause)
            }
            None => {
                let (path, mut generics) = Parser::parse2(
                    |input: ParseStream| {
                        Ok((Path::parse_mod_style(input)?, input.parse::<Generics>()?))
                    },
                    rest,
                )?;
                generics = with_where_predicates(generics);
                let ignored_params = ignored_params(&generics, &container_attributes)?;
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
                    let where_clause = &generics.where_clause;
                    let where_clause = quote! {#where_clause};
                    (quote! {#path}, generics, type_path, quote! {}, where_clause)
                } else {
                    let module_path = LitStr::new(&module_path, proc_macro2::Span::call_site());
                    let module_path = quote! {#module_path};

                    let (_, type_generics, _) = generics.split_for_impl();
                    let target = quote! {#path #type_generics};
                    let extended_where_clause = extend_where_clause(
                        &generics,
                        quote! {#bevy_reflect::Typed},
                        &ignored_params,
                    );
                    let (generic_type_path_impl, full_type_path, short_type_path) =
                        type_path_strings(
                            &target,
                            &generics,
                            &bevy_reflect,
                            &extended_where_clause,
                            &type_ident_str,
                            &module_path,
                            &ignored_params,
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
                        #bevy_reflect::kind::Kind::Primitive(#bevy_reflect::kind::Primitive::#primitive),
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
        #bevy_reflect::type_path::TypePath::new(
            #bevy_reflect::type_path::TypePathStr::Static(#full),
            #bevy_reflect::type_path::TypePathStr::Static(#short),
            #prefix_len..#type_ident_end,
            #crate_name,
            #module_path,
        )
    })
}
