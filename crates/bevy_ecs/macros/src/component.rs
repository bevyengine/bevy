use bevy_macro_utils::{get_lit_bool, get_lit_str, Symbol};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, DeriveInput, Error, Ident, Path, Result};

pub fn derive_resource(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let attrs = match parse_resource_attr(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let is_sync = attrs.is_sync;

    let where_clause = ast.generics.make_where_clause();
    where_clause
        .predicates
        .push(parse_quote! { Self: Send + 'static });

    // Add a bound to ensure that the `Sync`-ness is correct.
    if is_sync {
        where_clause.predicates.push(parse_quote! { Self: Sync });
    }

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        // SAFETY: The trait bounds on this impl guarantee that the non-`Sync` types will not have `IS_SYNC = true`.
        unsafe impl #impl_generics #bevy_ecs_path::system::Resource for #struct_name #type_generics #where_clause {
            const IS_SYNC: bool = #is_sync;
        }
    })
}

pub const RESOURCE: Symbol = Symbol("resource");
pub const IS_SYNC: Symbol = Symbol("is_sync");

struct ResourceAttrs {
    is_sync: bool,
}

fn parse_resource_attr(ast: &DeriveInput) -> Result<ResourceAttrs> {
    let meta_items = bevy_macro_utils::parse_attrs(ast, RESOURCE)?;

    let mut attrs = ResourceAttrs { is_sync: true };

    for meta in meta_items {
        use syn::{
            Meta::NameValue,
            NestedMeta::{Lit, Meta},
        };
        match meta {
            Meta(NameValue(m)) if m.path == IS_SYNC => {
                attrs.is_sync = get_lit_bool(IS_SYNC, &m.lit)?;
            }
            Meta(meta_item) => {
                return Err(Error::new_spanned(
                    meta_item.path(),
                    format!(
                        "unknown resource attribute `{}`",
                        meta_item.path().into_token_stream()
                    ),
                ));
            }
            Lit(lit) => {
                return Err(Error::new_spanned(
                    lit,
                    "unexpected literal in resource attribute",
                ))
            }
        }
    }

    Ok(attrs)
}

pub fn derive_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let attrs = match parse_component_attr(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let storage = storage_path(&bevy_ecs_path, attrs.storage);

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            type Storage = #storage;
        }
    })
}

pub const COMPONENT: Symbol = Symbol("component");
pub const STORAGE: Symbol = Symbol("storage");

struct Attrs {
    storage: StorageTy,
}

#[derive(Clone, Copy)]
enum StorageTy {
    Table,
    SparseSet,
}

// values for `storage` attribute
const TABLE: &str = "Table";
const SPARSE_SET: &str = "SparseSet";

fn parse_component_attr(ast: &DeriveInput) -> Result<Attrs> {
    let meta_items = bevy_macro_utils::parse_attrs(ast, COMPONENT)?;

    let mut attrs = Attrs {
        storage: StorageTy::Table,
    };

    for meta in meta_items {
        use syn::{
            Meta::NameValue,
            NestedMeta::{Lit, Meta},
        };
        match meta {
            Meta(NameValue(m)) if m.path == STORAGE => {
                attrs.storage = match get_lit_str(STORAGE, &m.lit)?.value().as_str() {
                    TABLE => StorageTy::Table,
                    SPARSE_SET => StorageTy::SparseSet,
                    s => {
                        return Err(Error::new_spanned(
                            m.lit,
                            format!(
                                "Invalid storage type `{}`, expected '{}' or '{}'.",
                                s, TABLE, SPARSE_SET
                            ),
                        ))
                    }
                };
            }
            Meta(meta_item) => {
                return Err(Error::new_spanned(
                    meta_item.path(),
                    format!(
                        "unknown component attribute `{}`",
                        meta_item.path().into_token_stream()
                    ),
                ));
            }
            Lit(lit) => {
                return Err(Error::new_spanned(
                    lit,
                    "unexpected literal in component attribute",
                ))
            }
        }
    }

    Ok(attrs)
}

fn storage_path(bevy_ecs_path: &Path, ty: StorageTy) -> TokenStream2 {
    let typename = match ty {
        StorageTy::Table => Ident::new("TableStorage", Span::call_site()),
        StorageTy::SparseSet => Ident::new("SparseStorage", Span::call_site()),
    };

    quote! { #bevy_ecs_path::component::#typename }
}
